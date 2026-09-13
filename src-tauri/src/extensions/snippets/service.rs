//! The keyword-expansion service: the key monitor, the bounded buffer, and the
//! expansion it triggers.
//!
//! The monitor delivers each keystroke as a `KeyStroke` on a channel. A worker
//! thread owns the buffer and does all the work off the input path: it matches,
//! reads snippets, and sends the backspaces and paste. The buffer is a small
//! ring cleared on every non-character key, a focus change, a pause, secure
//! input, and an excluded application, so it can never accumulate anything.

use std::collections::VecDeque;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use crate::extension::{ActivationResult, Service};
use crate::platform::{self, KeyMonitor, KeyStroke};
use crate::templates::{Template, Values};
use crate::text::TextTarget;

use super::matcher::{match_keyword, KeywordEntry};
use super::store::Records;

/// How long without a keystroke clears the buffer.
const PAUSE: Duration = Duration::from_secs(3);

/// Provides the applications the user has excluded, read fresh each keystroke so
/// a change to the list takes effect without a restart.
pub type ExcludedApps = Arc<dyn Fn() -> Vec<String> + Send + Sync>;

/// The recent-character buffer and the matching over it, kept pure so it can be
/// tested without a monitor.
struct Expander {
    buffer: VecDeque<char>,
    keywords: Vec<KeywordEntry>,
    /// The longest keyword plus one, so the character before a keyword is kept
    /// for the word-boundary check.
    max: usize,
}

struct Hit {
    id: String,
    keyword_chars: usize,
}

impl Expander {
    fn new() -> Self {
        Self {
            buffer: VecDeque::new(),
            keywords: Vec::new(),
            max: 0,
        }
    }

    fn set_keywords(&mut self, keywords: Vec<KeywordEntry>) {
        let longest = keywords
            .iter()
            .map(|entry| entry.keyword.chars().count())
            .max()
            .unwrap_or(0);
        self.max = if longest == 0 { 0 } else { longest + 1 };
        self.keywords = keywords;
        self.trim();
    }

    fn clear(&mut self) {
        self.buffer.clear();
    }

    fn trim(&mut self) {
        while self.buffer.len() > self.max {
            self.buffer.pop_front();
        }
    }

    /// Adds a character and returns a match if the buffer now ends with a
    /// keyword on a word boundary. On a match the buffer is cleared, so the same
    /// characters cannot expand twice.
    fn observe_char(&mut self, c: char) -> Option<Hit> {
        if self.max == 0 {
            return None;
        }
        self.buffer.push_back(c);
        self.trim();
        let text: String = self.buffer.iter().collect();
        let hit = match_keyword(&text, &self.keywords).map(|entry| Hit {
            id: entry.id.clone(),
            keyword_chars: entry.keyword.chars().count(),
        });
        if hit.is_some() {
            self.buffer.clear();
        }
        hit
    }
}

pub struct KeywordExpansion {
    records: Arc<Records>,
    text: Arc<dyn TextTarget>,
    excluded: ExcludedApps,
    running: Mutex<Option<Running>>,
}

struct Running {
    monitor: Box<dyn KeyMonitor>,
    worker: Option<JoinHandle<()>>,
}

impl KeywordExpansion {
    pub fn new(records: Arc<Records>, text: Arc<dyn TextTarget>, excluded: ExcludedApps) -> Self {
        Self {
            records,
            text,
            excluded,
            running: Mutex::new(None),
        }
    }
}

impl Service for KeywordExpansion {
    fn start(&self) -> ActivationResult {
        let (tx, rx) = mpsc::channel::<KeyStroke>();
        let monitor = match platform::start_key_monitor(Box::new(move |stroke| {
            let _ = tx.send(stroke);
        })) {
            Some(monitor) => monitor,
            None => return Err("keyword expansion needs a key monitor this platform lacks".into()),
        };

        let records = self.records.clone();
        let text = self.text.clone();
        let excluded = self.excluded.clone();
        let worker = std::thread::Builder::new()
            .name("dango-keyword-expansion".into())
            .spawn(move || run(rx, records, text, excluded))?;

        *self.running.lock().unwrap() = Some(Running {
            monitor,
            worker: Some(worker),
        });
        Ok(())
    }

    fn stop(&self) {
        if let Some(running) = self.running.lock().unwrap().take() {
            // Dropping the monitor stops the hook and drops the sender, which
            // disconnects the worker's channel and ends its loop.
            let Running { monitor, worker } = running;
            drop(monitor);
            if let Some(worker) = worker {
                let _ = worker.join();
            }
        }
    }
}

fn run(
    rx: mpsc::Receiver<KeyStroke>,
    records: Arc<Records>,
    text: Arc<dyn TextTarget>,
    excluded: ExcludedApps,
) {
    let mut expander = Expander::new();
    let mut last_app: Option<String> = None;
    loop {
        match rx.recv_timeout(PAUSE) {
            Ok(KeyStroke::Char(c)) => {
                let app = platform::foreground_app();
                if app != last_app {
                    expander.clear();
                    last_app = app.clone();
                }
                if platform::focused_field_is_secure() || is_excluded(app.as_deref(), &excluded) {
                    expander.clear();
                    continue;
                }
                expander.set_keywords(keyword_entries(&records));
                if let Some(hit) = expander.observe_char(c) {
                    expand(&records, text.as_ref(), &hit);
                }
            }
            Ok(KeyStroke::Clear) => expander.clear(),
            Err(RecvTimeoutError::Timeout) => expander.clear(),
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}

fn is_excluded(app: Option<&str>, excluded: &ExcludedApps) -> bool {
    let Some(app) = app else {
        return false;
    };
    excluded().iter().any(|name| name.eq_ignore_ascii_case(app))
}

fn keyword_entries(records: &Records) -> Vec<KeywordEntry> {
    records
        .all()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|record| {
            record.keyword.map(|keyword| KeywordEntry {
                id: record.id,
                keyword,
            })
        })
        .collect()
}

fn expand(records: &Records, text: &dyn TextTarget, hit: &Hit) {
    let Ok(record) = records.get(&hit.id) else {
        return;
    };
    let Ok(template) = Template::parse(&record.body) else {
        return;
    };
    let mut values = Values::default();
    if template.uses("clipboard") {
        values.clipboard = text.clipboard_text();
    }
    if template.uses("selection") {
        values.selection = text.selection().ok().flatten();
    }
    let Ok(rendered) = template.render(&values) else {
        return;
    };
    platform::note_insertion_target();
    // Held across the whole injection: the monitor must not observe the
    // backspaces and the paste it is about to send.
    let _injecting = platform::injection_guard();
    let _ = text.expand(hit.keyword_chars, &rendered.text, rendered.caret);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(pairs: &[(&str, &str)]) -> Vec<KeywordEntry> {
        pairs
            .iter()
            .map(|(id, keyword)| KeywordEntry {
                id: id.to_string(),
                keyword: keyword.to_string(),
            })
            .collect()
    }

    #[test]
    fn the_buffer_never_grows_past_the_longest_keyword_plus_one() {
        let mut expander = Expander::new();
        expander.set_keywords(entries(&[("s", "sig")]));
        for c in "the quick brown fox jumps".chars() {
            expander.observe_char(c);
        }
        assert!(expander.buffer.len() <= 4, "bounded to longest + 1");
    }

    #[test]
    fn a_keyword_expands_only_on_a_word_boundary_and_clears_after() {
        let mut expander = Expander::new();
        expander.set_keywords(entries(&[("s", "sig")]));
        // Inside a word: no match.
        for c in "design".chars() {
            assert!(expander.observe_char(c).is_none());
        }
        // After a space: matches, and the buffer is emptied so it cannot re-fire.
        let mut hit = None;
        for c in " sig".chars() {
            hit = expander.observe_char(c);
        }
        assert_eq!(hit.map(|h| h.id), Some("s".to_string()));
        assert_eq!(expander.buffer.len(), 0, "the match clears the buffer");
    }

    #[test]
    fn no_keywords_means_nothing_is_buffered() {
        let mut expander = Expander::new();
        expander.set_keywords(Vec::new());
        assert!(expander.observe_char('a').is_none());
        assert_eq!(expander.buffer.len(), 0);
    }

    #[test]
    fn clear_empties_the_buffer() {
        let mut expander = Expander::new();
        expander.set_keywords(entries(&[("s", "sig")]));
        expander.observe_char('s');
        expander.observe_char('i');
        expander.clear();
        assert_eq!(expander.buffer.len(), 0);
    }

    #[test]
    fn the_exclusion_check_is_case_insensitive() {
        let list: ExcludedApps = Arc::new(|| vec!["Proton Pass".to_string()]);
        assert!(is_excluded(Some("proton pass"), &list));
        assert!(!is_excluded(Some("Notepad"), &list));
        assert!(!is_excluded(None, &list));
    }
}
