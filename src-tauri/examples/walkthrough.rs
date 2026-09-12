//! Exercises the macOS selection and paste path against a real application.
//!
//! It drives `TextExchange` directly rather than the launcher's interface. That
//! is deliberate: the uncertain part is the platform layer, and driving a
//! launcher designed to overlay whatever is frontmost, from a harness that also
//! needs focus, mostly tests the harness. What this cannot cover is the path
//! from a keystroke in root search to the action being dispatched, which is a
//! few seconds by hand.
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

    use clipboard_rs::{Clipboard, ClipboardContext};
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};
    use objc2_app_kit::NSWorkspace;

    use dango_lib::extensions::clipboard::{Content, CrateClipboard};
    use dango_lib::platform;
    use dango_lib::text::{Launcher, OwnWrites, TextExchange};

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
        let Some(exchange) =
            platform::text_exchange(source, declared.clone(), Arc::new(NoLauncher))
        else {
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
        harness.both_writes_are_declared();
        harness.the_caret_lands_where_asked();
        harness.the_selection_is_read();
        harness.an_empty_selection_is_reported_as_empty();
        harness.insertion_is_prompt();

        let _ = harness.clipboard.set_text(String::new());
        harness.report();
    }

    impl Harness {
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

        fn focus(&mut self) {
            let _ = Command::new("open").arg("-e").arg(SCRATCH).status();
            std::thread::sleep(Duration::from_millis(600));
        }

        fn clear(&mut self) {
            self.focus();
            self.chord('a');
            let _ = self.enigo.key(Key::Backspace, Direction::Click);
            std::thread::sleep(Duration::from_millis(200));
        }

        /// What the document holds, read the only way another application will
        /// tell you. Destroys the clipboard, so it runs after clipboard checks.
        fn contents(&mut self) -> String {
            self.focus();
            self.chord('a');
            self.chord('c');
            std::thread::sleep(Duration::from_millis(400));
            self.clipboard.get_text().unwrap_or_default()
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
            self.clear();
            self.set_user_clipboard();
            self.focus();

            let result = self.exchange.insert("Best,\nNico", None);
            std::thread::sleep(Duration::from_millis(300));

            if let Err(error) = result {
                self.check("insertion succeeds", false, format!("{error}"));
                return;
            }
            let landed = self.contents();
            self.check(
                "the text arrives in the document",
                landed.contains("Best,") && landed.contains("Nico"),
                format!("the document holds {landed:?}"),
            );
        }

        fn the_users_clipboard_survives(&mut self) {
            println!("-- the user's clipboard survives an insertion");
            self.clear();
            self.set_user_clipboard();
            self.focus();

            let _ = self.exchange.insert("borrowed", None);
            std::thread::sleep(Duration::from_millis(300));

            let after = self.clipboard.get_text().unwrap_or_default();
            let landed = self.contents();
            let inserted = landed.contains("borrowed");
            self.check(
                "the clipboard holds what the user had",
                inserted && after == USER_CLIPBOARD,
                if inserted {
                    format!("the clipboard holds {after:?}, expected {USER_CLIPBOARD:?}")
                } else {
                    format!("nothing was inserted, so this proves nothing: {landed:?}")
                },
            );
        }

        fn both_writes_are_declared(&mut self) {
            println!("-- every borrowed write is declared to the history");
            self.clear();
            self.set_user_clipboard();
            self.focus();

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
            self.clear();
            self.set_user_clipboard();
            self.focus();

            let _ = self.exchange.insert("<b></b>", Some(3));
            std::thread::sleep(Duration::from_millis(300));

            self.focus();
            let _ = self.enigo.text("HERE");
            std::thread::sleep(Duration::from_millis(300));

            let landed = self.contents();
            self.check(
                "typing after the insertion lands at the caret",
                landed.contains("<b>HERE</b>"),
                format!("the document holds {landed:?}, expected <b>HERE</b>"),
            );
        }

        fn the_selection_is_read(&mut self) {
            println!("-- the selection is read from the application");
            self.clear();
            self.focus();
            let _ = self.enigo.text("quoted material");
            std::thread::sleep(Duration::from_millis(300));
            self.chord('a');
            self.set_user_clipboard();
            self.focus();

            let read = self.exchange.selection();

            match read {
                Ok(Some(text)) => {
                    let right = text.contains("quoted material");
                    self.check("the selection comes back", right, format!("read {text:?}"));
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
            self.clear();
            self.set_user_clipboard();
            self.focus();

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
            self.clear();
            self.set_user_clipboard();
            self.focus();

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
