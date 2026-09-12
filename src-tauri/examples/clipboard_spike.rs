//! M2 spike: whether the macOS clipboard source keeps a password out.
//!
//! The first run of this spike disproved two design decisions: Proton Pass sets
//! no privacy marker, and the frontmost application at the moment a change is
//! noticed is whatever the user switched to, not what they copied from. This
//! version exercises the answer, so the fix is confirmed rather than assumed.
//!
//! Usage: `cargo run --example clipboard_spike`

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("clipboard_spike only runs on macOS");
}

#[cfg(target_os = "macos")]
fn main() {
    spike::run();
}

#[cfg(target_os = "macos")]
mod spike {
    use std::time::Duration;

    use dango_lib::extensions::clipboard::DEFAULT_EXCLUDED_APPLICATIONS;
    use objc2_foundation::{NSDate, NSDefaultRunLoopMode, NSRunLoop};

    pub fn run() {
        // Registering for activations needs the main thread, which this is.
        let clipboard = dango_lib::platform_clipboard_source();

        println!("watching the clipboard, ctrl-c to stop\n");
        println!("copy something ordinary, then copy a password from Proton Pass");
        println!("and switch straight back, the way you actually would\n");

        // Polling goes on a background thread and the main thread runs the run
        // loop, which is the arrangement the real application has: workspace
        // notifications are delivered through the loop, so a main thread parked
        // in a sleep would never see an activation at all.
        std::thread::spawn(move || poll_forever(clipboard));

        let run_loop = NSRunLoop::currentRunLoop();
        loop {
            unsafe {
                run_loop.runMode_beforeDate(
                    NSDefaultRunLoopMode,
                    &NSDate::dateWithTimeIntervalSinceNow(0.25),
                );
            }
        }
    }

    fn poll_forever(
        clipboard: std::sync::Arc<dyn dango_lib::extensions::clipboard::ClipboardSource>,
    ) {
        let mut last = clipboard.sequence();
        loop {
            std::thread::sleep(Duration::from_millis(250));
            let now = clipboard.sequence();
            if now == last {
                continue;
            }
            last = now;

            // Exactly the order the watcher uses: markers, then attribution,
            // and content only if both allow it.
            let marked = clipboard.is_excluded();
            let candidates = clipboard.candidate_applications();
            let blocked: Vec<&str> = candidates
                .iter()
                .filter(|name| {
                    DEFAULT_EXCLUDED_APPLICATIONS
                        .iter()
                        .any(|excluded| excluded.eq_ignore_ascii_case(name))
                })
                .map(|name| name.as_str())
                .collect();

            println!("change {now}");
            println!("  candidates: {candidates:?}");
            println!("  marker says excluded: {marked}");

            if marked || !blocked.is_empty() {
                let reason = if marked {
                    "a privacy marker".to_string()
                } else {
                    format!("{blocked:?} is on the default exclusion list")
                };
                println!("  KEPT OUT by {reason}; reading nothing\n");
                continue;
            }

            match clipboard.text() {
                Some(text) => println!("  WOULD RECORD text: {:?}\n", clip(&text)),
                None => match clipboard.image() {
                    Some(bytes) => println!("  WOULD RECORD image: {} bytes\n", bytes.len()),
                    None => println!("  nothing readable\n"),
                },
            }
        }
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
