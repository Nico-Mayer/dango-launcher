//! Drives M5's macOS key services against a real application and a running
//! Dango, and checks what actually happened rather than what was sent.
//!
//! Two rules the earlier spike learned the hard way. Never assume `open -e`
//! focused the document: the frontmost application is verified before anything
//! is driven, or the probe reads the wrong window. And never let the process
//! that drives the keys be the one that answers whether they worked: the window
//! frame and the document text come back from System Events, a different
//! process, so a write that reported success while doing nothing is still
//! caught.
//!
//! A listen-only tap appended behind Dango's own acts as the witness for what
//! Dango did to an event. Being at the tail is the point: it sees the flags
//! after the hyperkey has added them, which is the one thing no application can
//! be asked about.
//!
//! Needs a running Dango with a `hyper+left` binding and a `;sig` snippet, and
//! the Accessibility permission on this binary.
//!
//! Usage: `cargo run --example macos_keys_walkthrough`

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("macos_keys_walkthrough only runs on macOS");
}

#[cfg(target_os = "macos")]
fn main() {
    harness::run();
}

#[cfg(target_os = "macos")]
mod harness {
    use std::ffi::c_void;
    use std::process::Command;
    use std::ptr::NonNull;
    use std::sync::{Mutex, OnceLock};
    use std::time::Duration;

    use objc2_core_foundation::{kCFRunLoopCommonModes, CFMachPort, CFRunLoop};
    use objc2_core_graphics::{
        CGEvent, CGEventField, CGEventFlags, CGEventSource, CGEventSourceStateID,
        CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventTapProxy, CGEventType,
    };

    const SCRATCH: &str = "/tmp/dango-keys-walkthrough.txt";

    /// F18, which the hyperkey's `hidutil` remap points CapsLock at.
    const HYPER: u16 = 79;
    const LEFT_ARROW: u16 = 123;
    const ESCAPE: i64 = 53;

    /// Long enough for a window move or a paste to land and redraw.
    const SETTLE: Duration = Duration::from_millis(400);
    /// Between synthesized keystrokes, so a target reading them keeps up.
    const KEY_GAP: Duration = Duration::from_millis(40);

    /// What the witness tap saw, newest last.
    #[derive(Clone, Copy, Debug)]
    struct Seen {
        kind: CGEventType,
        keycode: i64,
        flags: CGEventFlags,
    }

    fn witnessed() -> &'static Mutex<Vec<Seen>> {
        static SEEN: OnceLock<Mutex<Vec<Seen>>> = OnceLock::new();
        SEEN.get_or_init(|| Mutex::new(Vec::new()))
    }

    struct Harness {
        passed: usize,
        failed: Vec<String>,
    }

    impl Harness {
        fn check(&mut self, what: &str, ok: bool, detail: String) {
            if ok {
                self.passed += 1;
                println!("   ok   {what}   ({detail})");
            } else {
                self.failed.push(what.to_string());
                println!("   FAIL {what}   ({detail})");
            }
        }
    }

    pub fn run() {
        if !unsafe { objc2_application_services::AXIsProcessTrusted() } {
            eprintln!(
                "this harness needs the Accessibility permission on its own binary:\n  {}",
                std::env::current_exe().unwrap_or_default().display()
            );
            return;
        }
        if !dango_is_running() {
            eprintln!("no Dango is running; start one before this harness");
            return;
        }
        if !remap_is_installed() {
            eprintln!("the hyperkey's hidutil remap is not installed; is the hyperkey configured?");
            return;
        }
        start_witness();

        if !open_target() {
            eprintln!("could not bring a TextEdit window to the front");
            return;
        }

        let mut harness = Harness {
            passed: 0,
            failed: Vec::new(),
        };

        the_hyperkey_carries_its_flags(&mut harness);
        a_bound_chord_runs_its_command(&mut harness);
        a_solitary_tap_sends_the_tap_key(&mut harness);
        a_hold_sends_no_tap_key(&mut harness);
        a_keyword_expands_in_place(&mut harness);
        a_keyword_inside_a_word_does_not_expand(&mut harness);

        close_target();

        println!("\n{} passed, {} failed", harness.passed, harness.failed.len());
        for failure in &harness.failed {
            println!("  failed: {failure}");
        }
        if !harness.failed.is_empty() {
            std::process::exit(1);
        }
    }

    // -- the checks ---------------------------------------------------------

    /// The claim no application can answer: while the hyperkey is held, the
    /// events passing through carry the four hyper flags.
    fn the_hyperkey_carries_its_flags(harness: &mut Harness) {
        println!("-- the hyperkey adds its flags to an event held under it");
        witnessed().lock().unwrap().clear();
        hold_hyper_and_press(LEFT_ARROW);
        std::thread::sleep(SETTLE);

        let seen = witnessed().lock().unwrap().clone();
        let hyper = CGEventFlags::MaskControl
            | CGEventFlags::MaskAlternate
            | CGEventFlags::MaskShift
            | CGEventFlags::MaskCommand;
        let arrow = seen
            .iter()
            .find(|s| s.keycode == LEFT_ARROW as i64 && s.kind == CGEventType::KeyDown);

        harness.check(
            "the key pressed under the hyperkey carries Ctrl+Alt+Shift+Cmd",
            arrow.is_some_and(|s| s.flags.contains(hyper)),
            match arrow {
                Some(s) => format!("flags {:#x}, wanted {:#x} set", s.flags.0, hyper.0),
                None => "the witness never saw the arrow".into(),
            },
        );
        harness.check(
            "the hyperkey's own key is swallowed",
            !seen.iter().any(|s| s.keycode == HYPER as i64),
            format!("{} events witnessed, none for keycode {HYPER}", seen.len()),
        );
    }

    /// The end-to-end claim: the chord reaches the command and the command acts
    /// on the window that was focused, not on Dango.
    fn a_bound_chord_runs_its_command(harness: &mut Harness) {
        println!("-- a bound hyper chord runs its command on the focused window");
        if !frontmost_is("TextEdit") {
            harness.check(
                "hyper+left moves the focused window to the left half",
                false,
                "TextEdit was not frontmost, so nothing was driven".into(),
            );
            return;
        }

        let before = bounds();
        // Put the window somewhere that is definitely not the left half first,
        // so landing there cannot be where it already was.
        place(400, 200, 700, 500);
        std::thread::sleep(SETTLE);

        hold_hyper_and_press(LEFT_ARROW);
        std::thread::sleep(SETTLE);

        let after = bounds();
        let screen = work_area();
        let ok = match (after, screen) {
            (Some(after), Some(screen)) => {
                let half = screen.2 / 2.0;
                (after.0 - screen.0).abs() < 8.0
                    && (after.2 - half).abs() < 8.0
                    && after.3 > screen.3 * 0.8
            }
            _ => false,
        };
        harness.check(
            "hyper+left moves the focused window to the left half",
            ok,
            format!("before {before:?}, after {after:?}, work area {screen:?}"),
        );
    }

    fn a_solitary_tap_sends_the_tap_key(harness: &mut Harness) {
        println!("-- a quick solitary tap sends the configured tap key");
        witnessed().lock().unwrap().clear();
        post(HYPER, true, CGEventFlags::empty());
        std::thread::sleep(Duration::from_millis(60));
        post(HYPER, false, CGEventFlags::empty());
        std::thread::sleep(SETTLE);

        let seen = witnessed().lock().unwrap().clone();
        harness.check(
            "a 60ms solitary tap emits Escape",
            seen.iter()
                .any(|s| s.keycode == ESCAPE && s.kind == CGEventType::KeyDown),
            format!("witnessed {:?}", codes(&seen)),
        );
    }

    fn a_hold_sends_no_tap_key(harness: &mut Harness) {
        println!("-- a hold past the threshold sends no tap key");
        witnessed().lock().unwrap().clear();
        post(HYPER, true, CGEventFlags::empty());
        std::thread::sleep(Duration::from_millis(400));
        post(HYPER, false, CGEventFlags::empty());
        std::thread::sleep(SETTLE);

        let seen = witnessed().lock().unwrap().clone();
        harness.check(
            "a 400ms hold emits no Escape",
            !seen
                .iter()
                .any(|s| s.keycode == ESCAPE && s.kind == CGEventType::KeyDown),
            format!("witnessed {:?}", codes(&seen)),
        );
    }

    fn a_keyword_expands_in_place(harness: &mut Harness) {
        println!("-- typing a keyword replaces it with the snippet");
        if !focus_document(harness, "a keyword typed in a document expands in place") {
            return;
        }
        clear_document();
        type_text("hello ;sig");
        std::thread::sleep(Duration::from_millis(1200));

        let text = document_text().unwrap_or_default();
        harness.check(
            "a keyword typed in a document expands in place",
            !text.contains(";sig") && text.len() > "hello ".len(),
            format!("document holds {text:?}"),
        );
    }

    fn a_keyword_inside_a_word_does_not_expand(harness: &mut Harness) {
        println!("-- a keyword inside a longer word does not expand");
        if !focus_document(harness, "a keyword inside a word is left alone") {
            return;
        }
        clear_document();
        // No word boundary before the keyword, so it must not fire.
        type_text("xx;sig");
        std::thread::sleep(Duration::from_millis(1200));

        let text = document_text().unwrap_or_default();
        harness.check(
            "a keyword inside a word is left alone",
            text.contains(";sig"),
            format!("document holds {text:?}"),
        );
    }

    fn focus_document(harness: &mut Harness, what: &str) -> bool {
        if frontmost_is("TextEdit") {
            return true;
        }
        open_target();
        if frontmost_is("TextEdit") {
            return true;
        }
        harness.check(what, false, "TextEdit could not be brought to the front".into());
        false
    }

    // -- driving keys -------------------------------------------------------

    fn source() -> Option<objc2_core_foundation::CFRetained<CGEventSource>> {
        CGEventSource::new(CGEventSourceStateID::HIDSystemState)
    }

    fn post(keycode: u16, down: bool, flags: CGEventFlags) {
        let Some(event) = CGEvent::new_keyboard_event(source().as_deref(), keycode, down) else {
            return;
        };
        if !flags.is_empty() {
            CGEvent::set_flags(Some(&event), flags);
        }
        CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&event));
    }

    /// Presses the hyperkey, presses and releases another key under it, then
    /// releases the hyperkey - the shape of a real chord.
    fn hold_hyper_and_press(keycode: u16) {
        post(HYPER, true, CGEventFlags::empty());
        std::thread::sleep(KEY_GAP);
        post(keycode, true, CGEventFlags::empty());
        std::thread::sleep(KEY_GAP);
        post(keycode, false, CGEventFlags::empty());
        std::thread::sleep(KEY_GAP);
        post(HYPER, false, CGEventFlags::empty());
    }

    /// Types text a character at a time, carrying the character on the event
    /// rather than looking up a keycode, so the layout never comes into it.
    fn type_text(text: &str) {
        for character in text.chars() {
            let mut units = [0u16; 2];
            let units = character.encode_utf16(&mut units);
            for down in [true, false] {
                let Some(event) = CGEvent::new_keyboard_event(source().as_deref(), 0, down) else {
                    continue;
                };
                unsafe {
                    CGEvent::keyboard_set_unicode_string(
                        Some(&event),
                        units.len() as u64,
                        units.as_ptr(),
                    );
                }
                CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&event));
            }
            std::thread::sleep(KEY_GAP);
        }
    }

    // -- the witness --------------------------------------------------------

    /// A listen-only tap appended behind Dango's, so it observes each event as
    /// Dango left it.
    fn start_witness() {
        let mask = (1u64 << CGEventType::KeyDown.0) | (1u64 << CGEventType::KeyUp.0);
        std::thread::Builder::new()
            .name("witness".into())
            .spawn(move || {
                let tap = unsafe {
                    CGEvent::tap_create(
                        CGEventTapLocation::SessionEventTap,
                        CGEventTapPlacement::TailAppendEventTap,
                        CGEventTapOptions::ListenOnly,
                        mask,
                        Some(witness),
                        std::ptr::null_mut(),
                    )
                };
                let Some(tap) = tap else {
                    eprintln!("the witness tap was refused");
                    return;
                };
                let Some(source) = CFMachPort::new_run_loop_source(None, Some(&tap), 0) else {
                    return;
                };
                let Some(run_loop) = CFRunLoop::current() else {
                    return;
                };
                unsafe { run_loop.add_source(Some(&source), kCFRunLoopCommonModes) };
                CGEvent::tap_enable(&tap, true);
                CFRunLoop::run();
            })
            .expect("witness thread");
        std::thread::sleep(Duration::from_millis(300));
    }

    unsafe extern "C-unwind" fn witness(
        _proxy: CGEventTapProxy,
        kind: CGEventType,
        event: NonNull<CGEvent>,
        _user: *mut c_void,
    ) -> *mut CGEvent {
        let event_ref = unsafe { event.as_ref() };
        if kind == CGEventType::TapDisabledByTimeout || kind == CGEventType::TapDisabledByUserInput
        {
            return event.as_ptr();
        }
        let seen = Seen {
            kind,
            keycode: CGEvent::integer_value_field(
                Some(event_ref),
                CGEventField::KeyboardEventKeycode,
            ),
            flags: CGEvent::flags(Some(event_ref)),
        };
        if let Ok(mut log) = witnessed().lock() {
            log.push(seen);
        }
        event.as_ptr()
    }

    fn codes(seen: &[Seen]) -> Vec<i64> {
        seen.iter()
            .filter(|s| s.kind == CGEventType::KeyDown)
            .map(|s| s.keycode)
            .collect()
    }

    // -- the target ---------------------------------------------------------

    fn open_target() -> bool {
        let _ = std::fs::write(SCRATCH, "");
        let _ = Command::new("open").args(["-a", "TextEdit", SCRATCH]).status();
        for _ in 0..20 {
            std::thread::sleep(Duration::from_millis(250));
            if frontmost_is("TextEdit") {
                std::thread::sleep(SETTLE);
                return true;
            }
        }
        false
    }

    fn close_target() {
        let _ = osascript("tell application \"System Events\" to keystroke \"w\" using command down");
    }

    /// The spike's lesson: confirm which application is frontmost before
    /// driving it, rather than trusting that `open` did what was asked.
    fn frontmost_is(process: &str) -> bool {
        osascript(
            "tell application \"System Events\" to get name of first application process \
             whose frontmost is true",
        )
        .is_some_and(|name| name.trim() == process)
    }

    fn document_text() -> Option<String> {
        osascript(
            "tell application \"System Events\" to tell process \"TextEdit\" to get value of \
             text area 1 of scroll area 1 of window 1",
        )
    }

    fn clear_document() {
        let _ = osascript(
            "tell application \"System Events\" to tell process \"TextEdit\" to set value of \
             text area 1 of scroll area 1 of window 1 to \"\"",
        );
        std::thread::sleep(Duration::from_millis(200));
    }

    fn place(x: i32, y: i32, width: i32, height: i32) {
        let window = "(first window whose subrole is \"AXStandardWindow\")";
        let _ = osascript(&format!(
            "tell application \"System Events\" to tell process \"TextEdit\" to set \
             {{value of attribute \"AXPosition\" of {window}, \
               value of attribute \"AXSize\" of {window}}} to {{{{{x}, {y}}}, {{{width}, {height}}}}}"
        ));
    }

    fn bounds() -> Option<(f64, f64, f64, f64)> {
        let window = "(first window whose subrole is \"AXStandardWindow\")";
        let raw = osascript(&format!(
            "tell application \"System Events\" to tell process \"TextEdit\" to get \
             {{value of attribute \"AXPosition\" of {window}, \
               value of attribute \"AXSize\" of {window}}}",
        ))?;
        let numbers: Vec<f64> = raw
            .split(',')
            .filter_map(|part| part.trim().parse::<f64>().ok())
            .collect();
        match numbers[..] {
            [x, y, width, height] => Some((x, y, width, height)),
            _ => None,
        }
    }

    /// The visible frame of the main screen, which is what a half is a half of.
    fn work_area() -> Option<(f64, f64, f64, f64)> {
        let raw = osascript(
            "tell application \"Finder\" to get bounds of window of desktop",
        )?;
        let numbers: Vec<f64> = raw
            .split(',')
            .filter_map(|part| part.trim().parse::<f64>().ok())
            .collect();
        match numbers[..] {
            [left, top, right, bottom] => Some((left, top, right - left, bottom - top)),
            _ => None,
        }
    }

    fn dango_is_running() -> bool {
        Command::new("pgrep")
            .args(["-f", "Dango.app/Contents/MacOS/dango"])
            .output()
            .is_ok_and(|out| !out.stdout.is_empty())
    }

    fn remap_is_installed() -> bool {
        Command::new("hidutil")
            .args(["property", "--get", "UserKeyMapping"])
            .output()
            .is_ok_and(|out| {
                String::from_utf8_lossy(&out.stdout).contains("HIDKeyboardModifierMappingSrc")
            })
    }

    fn osascript(script: &str) -> Option<String> {
        let output = Command::new("osascript").args(["-e", script]).output().ok()?;
        if !output.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        (!text.is_empty()).then_some(text)
    }
}
