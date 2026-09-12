//! The watcher: the only thing in the product that reacts to what the user does
//! outside the launcher.
//!
//! It polls a counter rather than the clipboard itself. Reading the contents is
//! expensive and, on macOS, observable by other applications; a launcher should
//! not be seen touching the clipboard several times a second.

use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;

use super::history::{Bounds, Content, History};

/// The platform boundary. Everything about how a clipboard is read sits behind
/// this; above it there is only the question of whether to record.
pub trait ClipboardSource: Send + Sync {
    /// A counter that moves when the clipboard changes. Cheap enough to poll.
    fn sequence(&self) -> i64;
    /// Whether the current contents are marked as not for history. Asked before
    /// any content is read, so excluded content never enters memory.
    fn is_excluded(&self) -> bool;
    /// Every application that could have put the current contents there.
    ///
    /// Windows can name the owner exactly and answers with one. macOS cannot,
    /// and answers with every application that was frontmost across the
    /// interval the change fell in, because the spike showed that asking at the
    /// instant of noticing attributes a password to whatever the user switched
    /// to.
    fn candidate_applications(&self) -> Vec<String>;
    fn text(&self) -> Option<String>;
    /// The current contents as encoded PNG bytes, when there is an image.
    fn image(&self) -> Option<Vec<u8>>;
    fn set_text(&self, text: &str);
    fn set_image(&self, png: &[u8]);
}

/// What the watcher needs to know each time the clipboard moves. Read per
/// change rather than at startup, so editing the exclusion list takes effect
/// without a restart.
pub trait Policy: Send + Sync {
    fn bounds(&self) -> Bounds;
    /// Applications whose clipboard is never recorded.
    fn excluded_applications(&self) -> Vec<String>;
}

pub struct Watcher {
    source: Arc<dyn ClipboardSource>,
    history: Arc<History>,
    policy: Arc<dyn Policy>,
    /// The last sequence the watcher has accounted for. Starts at whatever is
    /// already on the clipboard, so starting Dango does not record what was
    /// copied before it.
    seen: AtomicI64,
    /// A sequence produced by Dango's own write, which must not come back as a
    /// new entry. Compared by counter rather than by content, because content
    /// would also swallow the user genuinely re-copying the same thing.
    ours: AtomicI64,
    running: AtomicBool,
}

impl Watcher {
    pub fn new(
        source: Arc<dyn ClipboardSource>,
        history: Arc<History>,
        policy: Arc<dyn Policy>,
    ) -> Self {
        let seen = source.sequence();
        Self {
            source,
            history,
            policy,
            seen: AtomicI64::new(seen),
            ours: AtomicI64::new(i64::MIN),
            running: AtomicBool::new(false),
        }
    }

    pub fn running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn start(&self) {
        self.running.store(true, Ordering::SeqCst);
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Puts an entry back on the clipboard, remembering the sequence that
    /// produced so the watcher does not record its own write.
    pub fn restore(&self, content: &Content) {
        match content {
            Content::Text(text) => self.source.set_text(text),
            Content::Image(png) => self.source.set_image(png),
        }
        self.ours.store(self.source.sequence(), Ordering::SeqCst);
    }

    /// One pass. Answers whether it recorded anything, which is what the tests
    /// and the poll loop both want to know.
    pub fn poll(&self, now: i64) -> bool {
        let sequence = self.source.sequence();
        if sequence == self.seen.load(Ordering::SeqCst) {
            return false;
        }
        self.seen.store(sequence, Ordering::SeqCst);

        if sequence == self.ours.load(Ordering::SeqCst) {
            return false;
        }
        // Both checks run before a single byte of content is read.
        if self.source.is_excluded() || self.copied_from_excluded_application() {
            return false;
        }

        let Some(content) = self.read() else {
            return false;
        };
        match self.history.record(content, self.policy.bounds(), now) {
            Ok(()) => true,
            Err(error) => {
                eprintln!("[dango] could not record what was copied: {error}");
                false
            }
        }
    }

    /// Any candidate being excluded is enough. Deliberately asymmetric: this
    /// will sometimes decline to record something it could have kept, which is
    /// a nuisance, rather than keep a password, which is a liability.
    fn copied_from_excluded_application(&self) -> bool {
        let excluded = self.policy.excluded_applications();
        if excluded.is_empty() {
            return false;
        }
        self.source.candidate_applications().iter().any(|name| {
            excluded
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(name))
        })
    }

    /// Text first: an application that offers both usually means the text, and
    /// the text is the cheaper thing to keep.
    fn read(&self) -> Option<Content> {
        if let Some(text) = self.source.text().filter(|t| !t.trim().is_empty()) {
            return Some(Content::Text(text));
        }
        self.source.image().map(Content::Image)
    }
}

/// How often the counter is read. Short enough that a copy followed quickly by
/// another is still caught, long enough that the read costs nothing.
pub const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(250);

/// Runs the watcher on its own thread, so nothing it does can touch the
/// activation path. Stops when the extension is disabled.
pub struct WatchService {
    watcher: Arc<Watcher>,
}

impl WatchService {
    pub fn new(watcher: Arc<Watcher>) -> Self {
        Self { watcher }
    }
}

impl crate::extension::Service for WatchService {
    fn start(&self) -> crate::extension::ActivationResult {
        self.watcher.start();
        let watcher = self.watcher.clone();
        std::thread::spawn(move || {
            while watcher.running() {
                watcher.poll(crate::ranking::now_millis());
                std::thread::sleep(POLL_INTERVAL);
            }
        });
        Ok(())
    }

    fn stop(&self) {
        self.watcher.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeClipboard {
        sequence: AtomicI64,
        text: Mutex<Option<String>>,
        image: Mutex<Option<Vec<u8>>>,
        excluded: AtomicBool,
        applications: Mutex<Vec<String>>,
        /// Every read of content, so a test can prove excluded content was
        /// never looked at.
        reads: Mutex<Vec<&'static str>>,
    }

    impl FakeClipboard {
        fn copy_text(&self, value: &str) {
            *self.text.lock().unwrap() = Some(value.into());
            *self.image.lock().unwrap() = None;
            self.sequence.fetch_add(1, Ordering::SeqCst);
        }

        fn copy_image(&self, bytes: &[u8]) {
            *self.text.lock().unwrap() = None;
            *self.image.lock().unwrap() = Some(bytes.to_vec());
            self.sequence.fetch_add(1, Ordering::SeqCst);
        }

        fn from(&self, application: &str) {
            *self.applications.lock().unwrap() = vec![application.into()];
        }

        /// The user copied in one application and switched to another before
        /// the change was noticed, so both are candidates.
        fn copied_in_then_switched_to(&self, copied_in: &str, switched_to: &str) {
            *self.applications.lock().unwrap() = vec![copied_in.into(), switched_to.into()];
        }
    }

    impl ClipboardSource for FakeClipboard {
        fn sequence(&self) -> i64 {
            self.sequence.load(Ordering::SeqCst)
        }
        fn is_excluded(&self) -> bool {
            self.excluded.load(Ordering::SeqCst)
        }
        fn candidate_applications(&self) -> Vec<String> {
            self.applications.lock().unwrap().clone()
        }
        fn text(&self) -> Option<String> {
            self.reads.lock().unwrap().push("text");
            self.text.lock().unwrap().clone()
        }
        fn image(&self) -> Option<Vec<u8>> {
            self.reads.lock().unwrap().push("image");
            self.image.lock().unwrap().clone()
        }
        fn set_text(&self, value: &str) {
            self.copy_text(value);
        }
        fn set_image(&self, png: &[u8]) {
            self.copy_image(png);
        }
    }

    #[derive(Default)]
    struct FakePolicy {
        excluded: Mutex<Vec<String>>,
        bounds: Mutex<Bounds>,
    }

    impl Policy for FakePolicy {
        fn bounds(&self) -> Bounds {
            *self.bounds.lock().unwrap()
        }
        fn excluded_applications(&self) -> Vec<String> {
            self.excluded.lock().unwrap().clone()
        }
    }

    fn setup() -> (Arc<FakeClipboard>, Arc<FakePolicy>, Arc<History>, Watcher) {
        let clipboard = Arc::new(FakeClipboard::default());
        let policy = Arc::new(FakePolicy::default());
        let dir = std::env::temp_dir().join(format!("dango-watch-{}", uuid::Uuid::new_v4()));
        let history = Arc::new(History::new(
            Arc::new(crate::store::Store::in_memory().unwrap()),
            dir,
        ));
        let watcher = Watcher::new(clipboard.clone(), history.clone(), policy.clone());
        (clipboard, policy, history, watcher)
    }

    fn texts(history: &History) -> Vec<String> {
        history
            .entries()
            .unwrap()
            .into_iter()
            .filter_map(|e| e.text)
            .collect()
    }

    #[test]
    fn a_copy_is_recorded() {
        let (clipboard, _, history, watcher) = setup();
        clipboard.copy_text("hello");
        assert!(watcher.poll(1));
        assert_eq!(texts(&history), ["hello"]);
    }

    #[test]
    fn nothing_happens_while_the_clipboard_sits_still() {
        let (clipboard, _, history, watcher) = setup();
        clipboard.copy_text("hello");
        assert!(watcher.poll(1));
        assert!(!watcher.poll(2), "an unchanged counter is not a new entry");
        assert!(!watcher.poll(3));
        assert_eq!(history.entries().unwrap().len(), 1);
    }

    #[test]
    fn what_was_on_the_clipboard_before_dango_started_is_not_recorded() {
        let clipboard = Arc::new(FakeClipboard::default());
        clipboard.copy_text("copied before we were watching");

        let policy = Arc::new(FakePolicy::default());
        let dir = std::env::temp_dir().join(format!("dango-watch-{}", uuid::Uuid::new_v4()));
        let history = Arc::new(History::new(
            Arc::new(crate::store::Store::in_memory().unwrap()),
            dir,
        ));
        let watcher = Watcher::new(clipboard, history.clone(), policy);

        assert!(!watcher.poll(1));
        assert!(history.entries().unwrap().is_empty());
    }

    #[test]
    fn an_image_is_recorded() {
        let (clipboard, _, history, watcher) = setup();
        clipboard.copy_image(&[1, 2, 3]);
        assert!(watcher.poll(1));
        assert_eq!(
            history.entries().unwrap()[0].kind,
            super::super::Kind::Image
        );
    }

    #[test]
    fn excluded_content_is_never_even_read() {
        let (clipboard, _, history, watcher) = setup();
        clipboard.copy_text("a password");
        clipboard.excluded.store(true, Ordering::SeqCst);

        assert!(!watcher.poll(1));
        assert!(history.entries().unwrap().is_empty());
        assert!(
            clipboard.reads.lock().unwrap().is_empty(),
            "excluded content must not reach Dango's memory at all"
        );
    }

    #[test]
    fn content_from_an_excluded_application_is_never_even_read() {
        let (clipboard, policy, history, watcher) = setup();
        policy.excluded.lock().unwrap().push("1Password".into());
        clipboard.from("1Password");
        clipboard.copy_text("a password");

        assert!(!watcher.poll(1));
        assert!(history.entries().unwrap().is_empty());
        assert!(clipboard.reads.lock().unwrap().is_empty());
    }

    #[test]
    fn the_exclusion_list_is_read_per_change_not_at_startup() {
        let (clipboard, policy, history, watcher) = setup();
        clipboard.from("Notes");
        clipboard.copy_text("first");
        assert!(watcher.poll(1));

        // Added while the watcher is already running.
        policy.excluded.lock().unwrap().push("Notes".into());
        clipboard.copy_text("second");
        assert!(!watcher.poll(2));

        assert_eq!(texts(&history), ["first"]);
    }

    /// The case the macOS spike exposed: a password copied from a manager the
    /// user leaves immediately, which attribution at an instant blamed on
    /// whatever they switched to.
    #[test]
    fn an_excluded_application_left_before_the_copy_was_noticed_still_excludes() {
        let (clipboard, policy, history, watcher) = setup();
        policy.excluded.lock().unwrap().push("Proton Pass".into());
        clipboard.copied_in_then_switched_to("Proton Pass", "Ghostty");
        clipboard.copy_text("a password");

        assert!(!watcher.poll(1));
        assert!(history.entries().unwrap().is_empty());
        assert!(
            clipboard.reads.lock().unwrap().is_empty(),
            "the password must not be read just because the user switched away"
        );
    }

    #[test]
    fn switching_between_two_allowed_applications_still_records() {
        let (clipboard, policy, history, watcher) = setup();
        policy.excluded.lock().unwrap().push("Proton Pass".into());
        clipboard.copied_in_then_switched_to("Notes", "Ghostty");
        clipboard.copy_text("something ordinary");

        assert!(watcher.poll(1));
        assert_eq!(texts(&history), ["something ordinary"]);
    }

    #[test]
    fn an_application_name_is_matched_regardless_of_case() {
        let (clipboard, policy, history, watcher) = setup();
        policy.excluded.lock().unwrap().push("1password".into());
        clipboard.from("1Password");
        clipboard.copy_text("a password");

        assert!(!watcher.poll(1));
        assert!(history.entries().unwrap().is_empty());
    }

    #[test]
    fn dangos_own_write_is_not_recorded() {
        let (clipboard, _, history, watcher) = setup();
        clipboard.copy_text("hello");
        assert!(watcher.poll(1));

        watcher.restore(&Content::Text("hello".into()));
        assert!(
            !watcher.poll(2),
            "putting an entry back must not create another entry"
        );
        assert_eq!(history.entries().unwrap().len(), 1);
    }

    #[test]
    fn the_user_recopying_the_same_thing_still_counts() {
        let (clipboard, _, history, watcher) = setup();
        clipboard.copy_text("hello");
        assert!(watcher.poll(1));
        watcher.restore(&Content::Text("hello".into()));
        assert!(!watcher.poll(2));

        // The user copies it again from somewhere else.
        clipboard.copy_text("hello");
        assert!(
            watcher.poll(3),
            "only Dango's own write is skipped, not the content"
        );
        assert_eq!(history.entries().unwrap().len(), 1, "and it deduplicates");
    }

    #[test]
    fn whitespace_only_text_is_not_worth_keeping() {
        let (clipboard, _, history, watcher) = setup();
        clipboard.copy_text("   \n  ");
        assert!(!watcher.poll(1));
        assert!(history.entries().unwrap().is_empty());
    }

    #[test]
    fn the_watcher_stops_when_it_is_told_to() {
        let (_, _, _, watcher) = setup();
        assert!(!watcher.running());
        watcher.start();
        assert!(watcher.running());
        watcher.stop();
        assert!(!watcher.running());
    }

    #[test]
    fn the_service_polls_until_it_is_stopped() {
        use crate::extension::Service;
        let (clipboard, _, history, watcher) = setup();
        let watcher = Arc::new(watcher);
        let service = WatchService::new(watcher.clone());

        service.start().unwrap();
        clipboard.copy_text("while running");
        std::thread::sleep(POLL_INTERVAL * 3);
        assert_eq!(texts(&history), ["while running"]);

        service.stop();
        // Long enough that a still-running loop would have polled again.
        std::thread::sleep(POLL_INTERVAL * 3);
        clipboard.copy_text("after stopping");
        std::thread::sleep(POLL_INTERVAL * 3);
        assert_eq!(
            texts(&history),
            ["while running"],
            "a stopped watcher records nothing"
        );
        assert!(!watcher.running());
    }
}
