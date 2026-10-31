//! M2 spike, Windows half: does the clipboard owner name the copying
//! application, and does a password manager set an exclusion format?
//!
//! Content is never printed unless `DANGO_SPIKE_SHOW_TEXT` is set, so the run
//! against a password manager cannot leak the password into a log.
//!
//! Usage: `cargo run --example clipboard_spike`

#[cfg(not(windows))]
fn main() {
    eprintln!("clipboard_spike only runs on Windows");
}

#[cfg(windows)]
fn main() {
    spike::run();
}

#[cfg(windows)]
mod spike {
    use std::sync::Arc;
    use std::time::Instant;

    use clipboard_rs::{ClipboardHandler, ClipboardWatcher, ClipboardWatcherContext};
    use dango_lib::extensions::clipboard::{
        Attribution, ClipboardSource, CrateClipboard, DEFAULT_EXCLUDED_APPLICATIONS,
    };
    use windows::Win32::System::DataExchange::GetClipboardSequenceNumber;

    const MARKERS: &[&str] = &[
        "ExcludeClipboardContentFromMonitorProcessing",
        "CanIncludeInClipboardHistory",
    ];

    struct Probe {
        clipboard: Arc<CrateClipboard>,
        attribution: Arc<dyn Attribution>,
        last_sequence: u32,
        last_event: Instant,
        events: u32,
        show_text: bool,
    }

    impl ClipboardHandler for Probe {
        fn on_clipboard_change(&mut self) {
            let arrived = Instant::now();
            let sequence = unsafe { GetClipboardSequenceNumber() };
            self.events += 1;
            println!(
                "event {} at +{:.0}ms: sequence {} (+{})",
                self.events,
                arrived.duration_since(self.last_event).as_secs_f64() * 1000.0,
                sequence,
                sequence.wrapping_sub(self.last_sequence)
            );
            self.last_sequence = sequence;
            self.last_event = arrived;

            let started = Instant::now();
            let formats = self.clipboard.formats();
            let formats_took = started.elapsed();
            let marked = formats.iter().any(|f| MARKERS.contains(&f.as_str()));

            let started = Instant::now();
            let candidates = self.attribution.candidate_applications();
            let owner_took = started.elapsed();
            let blocked: Vec<&str> = candidates
                .iter()
                .filter(|name| {
                    DEFAULT_EXCLUDED_APPLICATIONS
                        .iter()
                        .any(|excluded| excluded.eq_ignore_ascii_case(name))
                })
                .map(String::as_str)
                .collect();

            println!("  formats ({:.1}ms): {formats:?}", ms(formats_took));
            println!("  marker says excluded: {marked}");
            println!("  owner ({:.1}ms): {candidates:?}", ms(owner_took));

            if marked || !blocked.is_empty() {
                let reason = if marked {
                    "a privacy marker".to_string()
                } else {
                    format!("{blocked:?} is on the default exclusion list")
                };
                println!("  KEPT OUT by {reason}; reading nothing\n");
                return;
            }

            let started = Instant::now();
            match self.clipboard.text() {
                Some(text) if self.show_text => println!(
                    "  WOULD RECORD text ({:.1}ms): {:?}",
                    ms(started.elapsed()),
                    clip(&text)
                ),
                Some(text) => println!(
                    "  WOULD RECORD text ({:.1}ms): {} chars, content withheld",
                    ms(started.elapsed()),
                    text.chars().count()
                ),
                None => match self.clipboard.image() {
                    Some(png) => println!(
                        "  WOULD RECORD image ({:.1}ms): {} png bytes",
                        ms(started.elapsed()),
                        png.len()
                    ),
                    None => println!("  nothing readable ({:.1}ms)", ms(started.elapsed())),
                },
            }
            println!();
        }
    }

    pub fn run() {
        let probe = Probe {
            clipboard: CrateClipboard::new().expect("a clipboard"),
            attribution: dango_lib::platform_attribution(),
            last_sequence: unsafe { GetClipboardSequenceNumber() },
            last_event: Instant::now(),
            events: 0,
            show_text: std::env::var_os("DANGO_SPIKE_SHOW_TEXT").is_some(),
        };
        let mut watcher = ClipboardWatcherContext::new().expect("a watcher");
        watcher.add_handler(probe);
        println!("watching the clipboard, ctrl-c to stop\n");
        println!("copy something ordinary, then copy a password from a password manager\n");
        watcher.start_watch();
    }

    fn ms(duration: std::time::Duration) -> f64 {
        duration.as_secs_f64() * 1000.0
    }

    fn clip(value: &str) -> String {
        let flat: String = value
            .chars()
            .map(|c| if c == '\n' { '⏎' } else { c })
            .collect();
        if flat.chars().count() <= 60 {
            flat
        } else {
            format!("{}…", flat.chars().take(60).collect::<String>())
        }
    }
}
