//! Exercises the macOS selection and paste path against a real application.
//!
//! It drives `TextExchange` directly rather than the launcher's interface.
//!
//! Driving the interface was tried and abandoned twice. A synthesised
//! Option+Space never reaches the global shortcut, and launching the binary
//! again does open the launcher through the single-instance plugin but does not
//! deliver keystrokes to it: the typing goes nowhere the harness can observe,
//! and Enter produces no action. The panel is non-activating, so there is also
//! no asking whether it is up, because the frontmost application never changes.
//!
//! So the path from a keystroke in root search to an action stays a manual
//! check. Everything below the action is covered here.
//!
//! It synthesises keystrokes, so it needs the Accessibility permission and must
//! not be interrupted. A scratch TextEdit document is the target; nothing else
//! is touched, and the clipboard is put back.
//!
//! Usage: `cargo run --example walkthrough`

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("walkthrough only runs on macOS");
}

#[cfg(target_os = "macos")]
fn main() {
    harness::run();
}

#[cfg(target_os = "macos")]
mod harness {
    use std::process::Command;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use clipboard_rs::common::RustImage;
    use clipboard_rs::{Clipboard, ClipboardContext};
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};
    use objc2_app_kit::NSWorkspace;

    use dango_lib::extensions::clipboard::{Content, CrateClipboard};
    use dango_lib::platform;
    use dango_lib::text::{HereIsFine, Launcher, OwnWrites, TextExchange};

    const SCRATCH: &str = "/tmp/dango-walkthrough.txt";
    const USER_CLIPBOARD: &str = "dango-walkthrough-user-clipboard";

    /// The launcher is not involved here, so there is nothing to dismiss.
    struct NoLauncher;

    impl Launcher for NoLauncher {
        fn dismiss(&self) {}
    }

    /// Records what the exchange declares it is about to write, which is what
    /// keeps all of this out of the clipboard history.
    #[derive(Default)]
    struct Declared(std::sync::Mutex<Vec<Content>>);

    impl OwnWrites for Declared {
        fn expect(&self, content: &Content) {
            self.0.lock().unwrap().push(content.clone());
        }
    }

    impl Declared {
        fn count(&self) -> usize {
            self.0.lock().unwrap().len()
        }
    }

    struct Harness {
        enigo: Enigo,
        clipboard: ClipboardContext,
        exchange: TextExchange,
        declared: Arc<Declared>,
        passed: usize,
        failed: Vec<String>,
    }

    pub fn run() {
        if !unsafe { objc2_application_services::AXIsProcessTrusted() } {
            eprintln!("this needs Accessibility. Grant it to:");
            eprintln!("  {}", std::env::current_exe().unwrap().display());
            eprintln!("in System Settings > Privacy & Security > Accessibility, then rerun.");
            return;
        }

        let Some(source) = CrateClipboard::new() else {
            eprintln!("no clipboard");
            return;
        };
        let declared = Arc::new(Declared::default());
        let Some(exchange) = platform::text_exchange(
            source,
            declared.clone(),
            Arc::new(NoLauncher),
            // The harness runs everything on the main thread already.
            Arc::new(HereIsFine),
        ) else {
            eprintln!("no key injection");
            return;
        };
        let Ok(clipboard) = ClipboardContext::new() else {
            eprintln!("no clipboard");
            return;
        };
        let settings = Settings {
            open_prompt_to_get_permissions: false,
            independent_of_keyboard_state: true,
            ..Default::default()
        };
        let Ok(enigo) = Enigo::new(&settings) else {
            eprintln!("no key injection");
            return;
        };

        let mut harness = Harness {
            enigo,
            clipboard,
            exchange,
            declared,
            passed: 0,
            failed: vec![],
        };

        println!("This takes over the keyboard for about half a minute and opens");
        println!("a scratch TextEdit document. Ctrl-C now to abort.");
        for remaining in (1..=5).rev() {
            println!("  starting in {remaining}...");
            std::thread::sleep(Duration::from_secs(1));
        }
        println!();

        harness.open_scratch();
        if !harness.target_is_ready() {
            return;
        }

        harness.text_reaches_the_application();
        harness.the_users_clipboard_survives();
        harness.an_image_on_the_clipboard_survives();
        harness.both_writes_are_declared();
        harness.the_caret_lands_where_asked();
        harness.the_selection_is_read();
        harness.an_empty_selection_is_reported_as_empty();
        harness.insertion_is_prompt();

        let _ = harness.clipboard.set_text(String::new());
        harness.report();
    }

    impl Harness {
        /// Truncated hard. A failing check must describe what it saw without
        /// being able to print a page of whatever was on screen.
        fn brief(text: &str) -> String {
            let cleaned: String = text.chars().take(80).collect();
            if text.chars().count() > 80 {
                format!("{cleaned:?}... ({} chars total)", text.chars().count())
            } else {
                format!("{cleaned:?}")
            }
        }

        fn check(&mut self, name: &str, ok: bool, detail: String) {
            if ok {
                println!("  PASS  {name}");
                self.passed += 1;
            } else {
                println!("  FAIL  {name}\n          {detail}");
                self.failed.push(format!("{name}: {detail}"));
            }
        }

        fn open_scratch(&mut self) {
            // Rewriting the file does not reset a window TextEdit already has
            // open with unsaved changes, so the previous run's text survives
            // into this one. Quitting first is the only reliable reset.
            let _ = Command::new("osascript")
                .args(["-e", "tell application \"TextEdit\" to quit saving no"])
                .status();
            std::thread::sleep(Duration::from_millis(1200));
            let _ = std::fs::write(SCRATCH, "");
            let _ = Command::new("open").arg("-e").arg(SCRATCH).status();
            std::thread::sleep(Duration::from_millis(2000));
        }

        fn target_is_ready(&mut self) -> bool {
            let front = frontmost();
            if front != "TextEdit" {
                eprintln!("TextEdit is not frontmost ({front}); cannot drive it. Aborting.");
                return false;
            }
            println!("target: {front}\n");
            true
        }

        /// Returns whether TextEdit is genuinely frontmost. `open -e` does not
        /// always bring it forward, and assuming it did is how this harness
        /// twice read from a browser and printed the author's own screen.
        /// Nothing may be read without this answering true.
        fn focus(&mut self) -> bool {
            let _ = Command::new("open").arg("-e").arg(SCRATCH).status();
            for _ in 0..10 {
                std::thread::sleep(Duration::from_millis(200));
                if frontmost() == "TextEdit" {
                    return true;
                }
            }
            false
        }

        fn clear(&mut self) -> bool {
            if !self.focus() {
                return false;
            }
            self.chord('a');
            let _ = self.enigo.key(Key::Backspace, Direction::Click);
            std::thread::sleep(Duration::from_millis(200));
            true
        }

        /// What the document holds, read the only way another application will
        /// tell you. Destroys the clipboard, so it runs after clipboard checks.
        /// Reads the document by selecting and copying it, which means it must
        /// never run against the wrong window. Returns `None` rather than
        /// whatever happened to be frontmost.
        fn contents(&mut self) -> Option<String> {
            if !self.focus() {
                return None;
            }
            // An insertion ends by restoring the clipboard, and the sentinel
            // below is another clipboard write. Reading straight after one
            // races the other, which is what made the first check fail while
            // the text was in fact there.
            std::thread::sleep(Duration::from_millis(300));

            // The same ambiguity the exchange itself has to solve: copying an
            // empty document leaves the clipboard alone, so without a sentinel
            // an empty document reads back as whatever was already there.
            let sentinel = format!("dango-empty-{}", std::process::id());
            let _ = self.clipboard.set_text(sentinel.clone());
            std::thread::sleep(Duration::from_millis(150));

            self.chord('a');
            self.chord('c');
            std::thread::sleep(Duration::from_millis(400));
            let read = self.clipboard.get_text().unwrap_or_default();
            Some(if read == sentinel {
                String::new()
            } else {
                read
            })
        }

        fn chord(&mut self, letter: char) {
            let _ = self.enigo.key(Key::Meta, Direction::Press);
            let _ = self.enigo.key(Key::Unicode(letter), Direction::Click);
            let _ = self.enigo.key(Key::Meta, Direction::Release);
            std::thread::sleep(Duration::from_millis(150));
        }

        fn set_user_clipboard(&mut self) {
            let _ = self.clipboard.set_text(USER_CLIPBOARD.to_string());
            std::thread::sleep(Duration::from_millis(150));
        }

        fn text_reaches_the_application(&mut self) {
            println!("-- text reaches the frontmost application");
            if !self.clear() {
                self.check("TextEdit is drivable", false, "could not focus it".into());
                return;
            }
            self.set_user_clipboard();
            if !self.focus() {
                return;
            }

            let result = self.exchange.insert("Best,\nNico", None);
            if let Err(error) = result {
                self.check("insertion succeeds", false, format!("{error}"));
                return;
            }

            let Some(landed) = self.contents() else {
                self.check(
                    "the document could be read back",
                    false,
                    "TextEdit was not frontmost".into(),
                );
                return;
            };
            self.check(
                "the text arrives in the document",
                landed.contains("Best,") && landed.contains("Nico"),
                format!("the document holds {}", Self::brief(&landed)),
            );
        }

        fn the_users_clipboard_survives(&mut self) {
            println!("-- the user's clipboard survives an insertion");
            if !self.clear() {
                self.check("TextEdit is drivable", false, "could not focus it".into());
                return;
            }
            self.set_user_clipboard();
            if !self.focus() {
                return;
            }

            let _ = self.exchange.insert("borrowed", None);
            std::thread::sleep(Duration::from_millis(300));

            let after = self.clipboard.get_text().unwrap_or_default();
            let Some(landed) = self.contents() else {
                self.check(
                    "the document could be read back",
                    false,
                    "TextEdit was not frontmost".into(),
                );
                return;
            };
            let inserted = landed.contains("borrowed");
            self.check(
                "the clipboard holds what the user had",
                inserted && after == USER_CLIPBOARD,
                if inserted {
                    format!("the clipboard holds {after:?}, expected {USER_CLIPBOARD:?}")
                } else {
                    format!("nothing was inserted: {}", Self::brief(&landed))
                },
            );
        }

        /// A different branch from text: the exchange reads an image when there
        /// is no text, and has to put the image back rather than nothing.
        fn an_image_on_the_clipboard_survives(&mut self) {
            println!("-- an image on the clipboard survives an insertion");
            if !self.clear() {
                self.check("TextEdit is drivable", false, "could not focus it".into());
                return;
            }

            let png = std::fs::read(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../static/favicon.png"
            ))
            .expect("the repo's favicon");
            let Ok(image) = clipboard_rs::RustImageData::from_bytes(&png) else {
                self.check(
                    "an image can be put on the clipboard",
                    false,
                    "decode failed".into(),
                );
                return;
            };
            if self.clipboard.set_image(image).is_err() {
                self.check(
                    "an image can be put on the clipboard",
                    false,
                    "set failed".into(),
                );
                return;
            }
            std::thread::sleep(Duration::from_millis(200));
            let before = self
                .clipboard
                .get_image()
                .ok()
                .and_then(|image| image.to_png().ok())
                .map(|png| png.get_bytes().len());

            if !self.focus() {
                return;
            }
            let _ = self.exchange.insert("over an image", None);
            std::thread::sleep(Duration::from_millis(300));

            let after = self
                .clipboard
                .get_image()
                .ok()
                .and_then(|image| image.to_png().ok())
                .map(|png| png.get_bytes().len());

            self.check(
                "the image is still on the clipboard afterwards",
                after.is_some() && after == before,
                format!("held {before:?} bytes of PNG before, {after:?} after"),
            );

            let Some(landed) = self.contents() else {
                self.check(
                    "the document could be read back",
                    false,
                    "TextEdit was not frontmost".into(),
                );
                return;
            };
            self.check(
                "and the text still arrived",
                landed.contains("over an image"),
                format!("the document holds {}", Self::brief(&landed)),
            );
        }

        fn both_writes_are_declared(&mut self) {
            println!("-- every borrowed write is declared to the history");
            if !self.clear() {
                self.check("TextEdit is drivable", false, "could not focus it".into());
                return;
            }
            self.set_user_clipboard();
            if !self.focus() {
                return;
            }

            let before = self.declared.count();
            let _ = self.exchange.insert("declared", None);
            std::thread::sleep(Duration::from_millis(300));
            let declared = self.declared.count() - before;

            self.check(
                "the insertion declares its writes",
                declared == 2,
                format!("declared {declared} writes, expected 2: the text out and the restore"),
            );
        }

        fn the_caret_lands_where_asked(&mut self) {
            println!("-- the caret lands where the template asked");
            if !self.clear() {
                self.check("TextEdit is drivable", false, "could not focus it".into());
                return;
            }
            self.set_user_clipboard();
            if !self.focus() {
                return;
            }

            let _ = self.exchange.insert("<b></b>", Some(3));
            std::thread::sleep(Duration::from_millis(300));

            if !self.focus() {
                return;
            }
            let _ = self.enigo.text("HERE");
            std::thread::sleep(Duration::from_millis(300));

            let Some(landed) = self.contents() else {
                self.check(
                    "the document could be read back",
                    false,
                    "TextEdit was not frontmost".into(),
                );
                return;
            };
            self.check(
                "typing after the insertion lands at the caret",
                landed.contains("<b>HERE</b>"),
                format!(
                    "the document holds {}, expected <b>HERE</b>",
                    Self::brief(&landed)
                ),
            );
        }

        fn the_selection_is_read(&mut self) {
            println!("-- the selection is read from the application");
            if !self.clear() {
                self.check("TextEdit is drivable", false, "could not focus it".into());
                return;
            }
            if !self.focus() {
                return;
            }
            let _ = self.enigo.text("quoted material");
            std::thread::sleep(Duration::from_millis(300));
            // The clipboard is set before selecting, and nothing refocuses
            // afterwards: reopening the document to bring it forward drops the
            // selection, which made this flake.
            self.set_user_clipboard();
            self.chord('a');
            std::thread::sleep(Duration::from_millis(250));

            let read = self.exchange.selection();

            match read {
                Ok(Some(text)) => {
                    // Case-insensitive: TextEdit autocapitalises what is typed
                    // into it, so the document does not hold what was sent.
                    let right = text.to_lowercase().contains("quoted material");
                    self.check(
                        "the selection comes back",
                        right,
                        format!("read {}", Self::brief(&text)),
                    );
                    let after = self.clipboard.get_text().unwrap_or_default();
                    self.check(
                        "reading the selection gives the clipboard back",
                        after == USER_CLIPBOARD,
                        format!("the clipboard holds {after:?}"),
                    );
                }
                Ok(None) => self.check(
                    "the selection comes back",
                    false,
                    "reported no selection, but the document was selected".into(),
                ),
                Err(error) => self.check("the selection comes back", false, format!("{error}")),
            }
        }

        fn an_empty_selection_is_reported_as_empty(&mut self) {
            println!("-- nothing selected is reported as nothing");
            if !self.clear() {
                self.check("TextEdit is drivable", false, "could not focus it".into());
                return;
            }
            self.set_user_clipboard();
            if !self.focus() {
                return;
            }

            let read = self.exchange.selection();

            self.check(
                "an empty document reports no selection",
                matches!(read, Ok(None)),
                format!("expected Ok(None), got {read:?}"),
            );
            let after = self.clipboard.get_text().unwrap_or_default();
            self.check(
                "and leaves the clipboard alone",
                after == USER_CLIPBOARD,
                format!("the clipboard holds {after:?}"),
            );
        }

        fn insertion_is_prompt(&mut self) {
            println!("-- the text appears promptly");
            if !self.clear() {
                self.check("TextEdit is drivable", false, "could not focus it".into());
                return;
            }
            self.set_user_clipboard();
            if !self.focus() {
                return;
            }

            let start = Instant::now();
            let _ = self.exchange.insert("prompt", None);
            let elapsed = start.elapsed();

            // The spec budgets 400ms for the text appearing. This call also
            // carries the clipboard restore, which the user never waits for, so
            // it is the pessimistic reading of that budget.
            self.check(
                "an insertion returns inside the budget",
                elapsed < Duration::from_millis(900),
                format!("insert took {elapsed:?}"),
            );
            println!("          (insert returned in {elapsed:?}, restore included)");
        }

        fn report(&self) {
            println!("\n{} passed, {} failed", self.passed, self.failed.len());
            for failure in &self.failed {
                println!("  {failure}");
            }
            println!("\nscratch document left at {SCRATCH}");
        }
    }

    fn frontmost() -> String {
        NSWorkspace::sharedWorkspace()
            .frontmostApplication()
            .and_then(|app| app.localizedName())
            .map(|name| name.to_string())
            .unwrap_or_else(|| "<unknown>".into())
    }
}
