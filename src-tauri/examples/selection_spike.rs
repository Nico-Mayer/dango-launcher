//! M3 spike, macOS half. Four questions the design is built on:
//!
//! 1.2 Which applications answer `AXSelectedText`, and which return nothing.
//! 1.3 How long after the paste keystroke the clipboard can safely be restored.
//! 1.4 How often Accessibility trust survives a rebuild of an unsigned binary.
//! 1.7 What `undeclared_variables` reports for the templates snippets will hold.
//!
//! Selected text is never printed unless `DANGO_SPIKE_SHOW_TEXT` is set, since
//! the selection may be anything the user had highlighted.
//!
//! Usage:
//!   `cargo run --example selection_spike`            all of the above
//!   `cargo run --example selection_spike -- watch`   1.2, once a second
//!   `cargo run --example selection_spike -- paste`   1.3, into the frontmost app

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("selection_spike only runs on macOS");
}

#[cfg(target_os = "macos")]
fn main() {
    spike::run();
}

#[cfg(target_os = "macos")]
mod spike {
    use std::ptr::NonNull;
    use std::time::{Duration, Instant};

    use objc2_app_kit::NSWorkspace;
    use objc2_application_services::{AXError, AXUIElement};
    use objc2_core_foundation::{CFRetained, CFString, CFType};
    use objc2_foundation::{NSDate, NSRunLoop};

    use dango_lib::extensions::clipboard::{ClipboardSource, CrateClipboard};

    const FOCUSED_ELEMENT: &str = "AXFocusedUIElement";
    const SELECTED_TEXT: &str = "AXSelectedText";

    pub fn run() {
        let mode = std::env::args().nth(1).unwrap_or_else(|| "all".into());
        report_trust();
        match mode.as_str() {
            "watch" => watch_selection(),
            "paste" => measure_paste(),
            _ => {
                read_selection_once();
                template_probe();
                println!(
                    "\nrun `cargo run --example selection_spike -- watch` to sweep applications,"
                );
                println!("and `-- paste` to measure how soon the clipboard can be restored.");
            }
        }
    }

    /// 1.4. Trust is per binary, and an unsigned debug build changes identity
    /// every time it is rebuilt, so this line is the thing to watch across runs.
    fn report_trust() {
        let trusted = unsafe { objc2_application_services::AXIsProcessTrusted() };
        println!("accessibility trusted: {trusted}");
        if !trusted {
            println!(
                "  grant it in System Settings > Privacy & Security > Accessibility,\n  \
                 then run this again. Nothing below will work until then."
            );
        }
    }

    fn frontmost_application() -> (String, i32) {
        NSWorkspace::sharedWorkspace()
            .frontmostApplication()
            .map(|app| {
                let name = app
                    .localizedName()
                    .map(|name| name.to_string())
                    .unwrap_or_else(|| "<unknown>".into());
                (name, app.processIdentifier())
            })
            .unwrap_or_else(|| ("<unknown>".into(), -1))
    }

    /// The number alone says nothing. `-25204` is `CannotComplete`, which means
    /// the application did not answer, and is a completely different finding
    /// from `APIDisabled`, which would mean the permission.
    fn error_name(error: AXError) -> String {
        let name = match error {
            AXError::Failure => "Failure",
            AXError::IllegalArgument => "IllegalArgument",
            AXError::InvalidUIElement => "InvalidUIElement",
            AXError::CannotComplete => "CannotComplete (the app did not answer)",
            AXError::AttributeUnsupported => "AttributeUnsupported",
            AXError::NotImplemented => "NotImplemented (no accessibility support)",
            AXError::APIDisabled => "APIDisabled (the permission is off)",
            AXError::NoValue => "NoValue",
            _ => "unknown",
        };
        format!("{name} [{}]", error.0)
    }

    /// 1.2. System-wide element, focused element, selected text. Reports the
    /// error rather than flattening it, because "no selection" and "this
    /// application does not answer" are different findings.
    fn read_selection() -> Result<Option<String>, AXError> {
        unsafe { read_from(AXUIElement::new_system_wide()) }
    }

    /// The same question asked of the application directly rather than through
    /// the system-wide element. Worth measuring separately: if this answers
    /// where the system-wide route does not, the implementation should use it.
    fn read_selection_via_app(pid: i32) -> Result<Option<String>, AXError> {
        if pid < 0 {
            return Ok(None);
        }
        unsafe { read_from(AXUIElement::new_application(pid)) }
    }

    unsafe fn read_from(
        root: objc2_core_foundation::CFRetained<AXUIElement>,
    ) -> Result<Option<String>, AXError> {
        unsafe {
            let system = root;
            let focused = copy_attribute(&system, FOCUSED_ELEMENT)?;
            let Some(focused) = focused else {
                return Ok(None);
            };
            let focused = focused
                .downcast::<AXUIElement>()
                .map_err(|_| AXError(-25200))?;

            let selected = copy_attribute(&focused, SELECTED_TEXT)?;
            let Some(selected) = selected else {
                return Ok(None);
            };
            match selected.downcast::<CFString>() {
                Ok(text) => Ok(Some(text.to_string())),
                Err(_) => Ok(None),
            }
        }
    }

    unsafe fn copy_attribute(
        element: &AXUIElement,
        attribute: &str,
    ) -> Result<Option<CFRetained<CFType>>, AXError> {
        let name = CFString::from_str(attribute);
        let mut value: *const CFType = std::ptr::null();
        let status = unsafe { element.copy_attribute_value(&name, NonNull::from(&mut value)) };
        match status {
            AXError::Success => {
                if value.is_null() {
                    Ok(None)
                } else {
                    Ok(Some(unsafe {
                        CFRetained::from_raw(NonNull::new_unchecked(value.cast_mut()))
                    }))
                }
            }
            AXError::NoValue | AXError::AttributeUnsupported => Ok(None),
            other => Err(other),
        }
    }

    fn describe(result: Result<Option<String>, AXError>) -> String {
        match result {
            Ok(Some(text)) if text.is_empty() => "answered, but the selection is empty".into(),
            Ok(Some(text)) => {
                let shown = if std::env::var_os("DANGO_SPIKE_SHOW_TEXT").is_some() {
                    format!(" {text:?}")
                } else {
                    String::new()
                };
                format!("answered, {} chars{shown}", text.chars().count())
            }
            Ok(None) => "did not answer (no attribute, or nothing focused)".into(),
            Err(error) => error_name(error),
        }
    }

    fn read_selection_once() {
        println!("\n-- 1.2 selection, one read --");
        let (app, pid) = frontmost_application();
        println!("frontmost: {app}");
        // What a failing route costs matters as much as whether it works: it
        // sits in front of the clipboard fallback on the slowest path.
        for (label, read) in [
            (
                "system-wide",
                Box::new(read_selection) as Box<dyn Fn() -> _>,
            ),
            ("application", Box::new(move || read_selection_via_app(pid))),
        ] {
            let start = Instant::now();
            let result = read();
            println!(
                "  {label}:  {:>7.1?}  {}",
                start.elapsed(),
                describe(result)
            );
        }
    }

    /// Select text in one application, switch to the next, and let this print a
    /// line per second. The point is the list of which applications answer.
    fn watch_selection() {
        println!("\n-- 1.2 selection sweep, Ctrl-C to stop --");
        println!("select text in a native app, a browser, an Electron app, and a terminal.");
        let mut last = String::new();
        loop {
            let (app, pid) = frontmost_application();
            let line = format!(
                "{app}\n    system-wide: {}\n    application: {}",
                describe(read_selection()),
                describe(read_selection_via_app(pid))
            );
            if line != last {
                println!("{line}");
                last = line;
            }
            // Not a sleep. NSWorkspace only updates which application is
            // frontmost while a run loop is pumping, so sleeping here pins the
            // answer to whatever was in front when the probe started. The
            // clipboard change recorded this exact trap and it was walked into
            // again.
            NSRunLoop::currentRunLoop().runUntilDate(&NSDate::dateWithTimeIntervalSinceNow(1.0));
        }
    }

    /// 1.3. How soon after the paste keystroke the clipboard can be put back.
    ///
    /// The first version of this asked whether the clipboard still held what we
    /// wrote, which is always true: pasting does not change the clipboard. It
    /// measured nothing.
    ///
    /// This asks the only question that matters. Paste a marker, restore to a
    /// decoy after N milliseconds, then read the field back. If the marker is
    /// there, a restore at N was safe. If the decoy is there, the target had
    /// not read the clipboard yet and the user would have pasted the wrong
    /// thing.
    ///
    /// It types into whatever is focused, so give it an empty scratch field.
    fn measure_paste() {
        use enigo::{Enigo, Settings};

        println!("\n-- 1.3 paste and restore timing --");
        println!("focus an EMPTY editable field in another application. Starting in 5s.");
        println!("this will type into it and select all repeatedly.");
        std::thread::sleep(Duration::from_secs(5));

        let (target, _) = frontmost_application();
        let Some(clipboard) = CrateClipboard::new() else {
            eprintln!("no clipboard");
            return;
        };
        let saved = clipboard.text();

        let settings = Settings {
            open_prompt_to_get_permissions: false,
            ..Default::default()
        };
        let mut enigo = match Enigo::new(&settings) {
            Ok(enigo) => enigo,
            Err(error) => {
                eprintln!("enigo refused to start: {error:?}");
                return;
            }
        };

        println!("target: {target}");
        let mut safest: Option<u64> = None;
        for delay in [0u64, 5, 10, 20, 40, 80, 160] {
            let marker = format!("MARKER-{delay}");
            let decoy = format!("DECOY-{delay}");

            clipboard.set_text(&marker);
            chord(&mut enigo, 'a');
            let sent = Instant::now();
            chord(&mut enigo, 'v');

            std::thread::sleep(Duration::from_millis(delay));
            clipboard.set_text(&decoy);

            // Let the paste finish either way before reading the field back.
            std::thread::sleep(Duration::from_millis(400));
            chord(&mut enigo, 'a');
            chord(&mut enigo, 'c');
            std::thread::sleep(Duration::from_millis(120));

            let landed = clipboard.text().unwrap_or_default();
            let verdict = if landed.contains(&marker) {
                if safest.is_none() {
                    safest = Some(delay);
                }
                "safe: the marker landed"
            } else if landed.contains(&decoy) {
                "TOO EARLY: the decoy landed"
            } else {
                "unclear: neither landed"
            };
            println!(
                "  restore after {delay:>4}ms (keystroke {:>6.1?}): {verdict}",
                sent.elapsed()
            );
        }

        match saved {
            Some(text) => {
                clipboard.set_text(&text);
                println!("clipboard restored");
            }
            None => println!("nothing to restore"),
        }
        match safest {
            Some(delay) => println!(
                "\nearliest safe restore in this run: {delay}ms. \
                 Set PASTE_SETTLE above the worst seen, not this one."
            ),
            None => println!("\nno delay was safe; the approach needs rethinking"),
        }
        println!("clear the scratch field when you are done with it.");
    }

    fn chord(enigo: &mut enigo::Enigo, letter: char) {
        use enigo::{Direction, Key, Keyboard};
        let _ = enigo.key(Key::Meta, Direction::Press);
        let _ = enigo.key(Key::Unicode(letter), Direction::Click);
        let _ = enigo.key(Key::Meta, Direction::Release);
    }

    /// 1.7. The question the engine choice rests on: does the crate report the
    /// names a snippet references, before anything is rendered?
    fn template_probe() {
        use minijinja::Environment;

        println!("\n-- 1.7 undeclared_variables --");
        let cases = [
            (
                "reserved only",
                "Hi, today is {{ date }} and I copied {{ clipboard }}",
            ),
            ("one argument", "Dear {{ name }}, thanks."),
            (
                "repeated argument",
                "{{ city }} to {{ city }} via {{ stop }}",
            ),
            (
                "mixed",
                "{{ greeting }} {{ name }}, sent {{ date }}{{ cursor }}",
            ),
            (
                "single braces in code",
                "fn main() { let x = Foo { a: 1 }; }",
            ),
            (
                "doubled braces in code",
                "printf(\"{{%d}}\", x); if (a) {{ b(); }}",
            ),
            ("github actions", "runs-on: ${{ matrix.os }}"),
            ("vue interpolation", "<p>{{ user.name }}</p>"),
            (
                "raw block escape",
                "{% raw %}runs-on: ${{ matrix.os }}{% endraw %}",
            ),
            ("literal escape", "{{ '{{' }} matrix.os {{ '}}' }}"),
        ];
        let env = Environment::new();
        for (label, source) in cases {
            match env.template_from_str(source) {
                Ok(template) => {
                    let mut names: Vec<_> =
                        template.undeclared_variables(false).into_iter().collect();
                    names.sort();
                    println!("  {label}: {names:?}");
                }
                Err(error) => println!("  {label}: parse error: {error}"),
            }
        }
        let _ = env.template_from_str("unclosed {{ name");
        println!(
            "  unclosed placeholder parses: {}",
            env.template_from_str("unclosed {{ name").is_ok()
        );
    }
}
