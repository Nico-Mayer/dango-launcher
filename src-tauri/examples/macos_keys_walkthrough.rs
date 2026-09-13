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
    use std::cell::RefCell;
    use std::ffi::c_void;
    use std::process::Command;
    use std::ptr::NonNull;
    use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
    use std::sync::{Arc, Mutex, OnceLock};
    use std::time::{Duration, Instant};

    use objc2::rc::Retained;
    use objc2::{sel, MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{
        NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSEventMask, NSMenu,
        NSMenuItem, NSScrollView, NSTextView, NSWindow, NSWindowStyleMask,
    };
    use objc2_core_foundation::{kCFRunLoopCommonModes, CFMachPort, CFRunLoop};
    use objc2_core_graphics::{
        CGEvent, CGEventField, CGEventFlags, CGEventSource, CGEventSourceStateID,
        CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventTapProxy, CGEventType,
    };
    use objc2_foundation::{NSDate, NSDefaultRunLoopMode, NSPoint, NSRect, NSSize, NSString};

    /// F18, which the hyperkey's `hidutil` remap points CapsLock at.
    const HYPER: u16 = 79;
    const LEFT_ARROW: u16 = 123;
    /// K, which nothing is bound to, so the flag probe does not spend a press
    /// of a command that cycles through sizes on repeats.
    const UNBOUND: u16 = 40;
    const ESCAPE: i64 = 53;
    const ESCAPE_KEY: u16 = 53;
    const SPACE: u16 = 49;
    const F9: u16 = 38;

    /// Long enough for a window move or a paste to land and redraw.
    const SETTLE: Duration = Duration::from_millis(400);
    /// Between synthesized keystrokes, so a target reading them keeps up.
    const KEY_GAP: Duration = Duration::from_millis(60);

    /// What the witness tap saw, newest last.
    #[derive(Clone, Copy, Debug)]
    struct Seen {
        kind: CGEventType,
        keycode: i64,
        flags: CGEventFlags,
    }

    thread_local! {
        /// The scratch window and its text view, owned by the main thread
        /// because AppKit objects may only be touched there.
        static WINDOW: RefCell<Option<Retained<NSWindow>>> = const { RefCell::new(None) };
        static TEXT_VIEW: RefCell<Option<Retained<NSTextView>>> = const { RefCell::new(None) };
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

    /// The main thread builds the window and then does nothing but pump its
    /// event loop. Every check runs on a worker, because a check asks System
    /// Events about this very process, and System Events can only get an answer
    /// while this process is handling events. Blocking the main thread on that
    /// question is what makes it unanswerable.
    pub fn run() {
        if !preflight() {
            return;
        }
        start_witness();
        if !build_window() {
            eprintln!("could not build the scratch window");
            return;
        }

        let done = Arc::new(AtomicBool::new(false));
        let finished = done.clone();
        std::thread::Builder::new()
            .name("checks".into())
            .spawn(move || {
                checks();
                finished.store(true, Ordering::SeqCst);
            })
            .expect("checks thread");

        while !done.load(Ordering::SeqCst) {
            pump(Duration::from_millis(40));
        }
        pump(Duration::from_millis(100));
        std::process::exit(EXIT.load(Ordering::SeqCst));
    }

    /// The checks' own exit status, since they no longer run on the thread that
    /// returns from `main`.
    static EXIT: AtomicI32 = AtomicI32::new(0);

    fn preflight() -> bool {
        if !unsafe { objc2_application_services::AXIsProcessTrusted() } {
            eprintln!(
                "this harness needs the Accessibility permission on its own binary:\n  {}",
                std::env::current_exe().unwrap_or_default().display()
            );
            return false;
        }
        if !dango_is_running() {
            eprintln!("no Dango is running; start one before this harness");
            return false;
        }
        let mode = std::env::args().nth(1).unwrap_or_default();
        let drives_the_hyperkey = mode.is_empty() || mode == "no-tap";
        if drives_the_hyperkey && !remap_is_installed() {
            eprintln!("the hyperkey's hidutil remap is not installed; is the hyperkey configured?");
            return false;
        }
        true
    }

    fn checks() {
        let mode = std::env::args().nth(1).unwrap_or_default();
        if !focus_target() {
            eprintln!("could not bring the scratch window to the front");
            EXIT.store(1, Ordering::SeqCst);
            return;
        }

        let mut harness = Harness {
            passed: 0,
            failed: Vec::new(),
        };

        // Checks the launcher hotkey: the configured chord summons and the
        // platform default no longer does. The chords are fixed rather than
        // parsed from the file, so the script that sets the config and this
        // agree by construction.
        if mode == "launcher" || mode == "launcher-default" {
            // With a config: the configured chord works and the default does
            // not. With none: exactly the other way round.
            let expect_configured = mode == "launcher";
            let configured = summons(F9, CGEventFlags::MaskControl | CGEventFlags::MaskAlternate);
            dismiss();
            let default_chord = summons(SPACE, CGEventFlags::MaskAlternate);
            dismiss();

            harness.check(
                "the configured chord summons only when it is configured",
                configured == expect_configured,
                format!("Ctrl+Alt+J summoned: {configured}, expected {expect_configured}"),
            );
            harness.check(
                "the platform default summons only when nothing overrides it",
                default_chord == !expect_configured,
                format!(
                    "Option+Space summoned: {default_chord}, expected {}",
                    !expect_configured
                ),
            );
            report(&harness);
            return;
        }

        // Types whatever is asked into the scratch document and prints what
        // the document then holds. System Events' own `keystroke` cannot be
        // used for this: it posts to the target application directly, so a
        // session tap never sees it and the monitor is bypassed.
        // Presses Ctrl+Alt+<keycode> with the scratch document focused and
        // reports what happened: whether the launcher came up, and whether the
        // focused window moved. Enough to tell a no-view command from a view
        // command from an unbound chord.
        // Opens the scratch window and holds it up, so its state can be
        // inspected from another process while it is running.
        if mode == "hold" {
            raise_target();
            println!("process name: {:?}", own_process());
            pump(Duration::from_secs(25));
            return;
        }

        if mode == "chord" {
            let keycode: u16 = std::env::args()
                .nth(2)
                .and_then(|a| a.parse().ok())
                .unwrap_or(LEFT_ARROW);
            std::thread::sleep(Duration::from_secs(1));
            // Any launcher left up by a previous probe would be counted here.
            dismiss();
            // The window is not repositioned first: macOS 26 holds a tiled
            // window, and an accessibility write reports success while moving
            // nothing. The caller sets the window up with another bound command
            // instead, so every move in this check comes from Dango.
            let before = bounds();

            post(
                keycode,
                true,
                CGEventFlags::MaskControl | CGEventFlags::MaskAlternate,
            );
            std::thread::sleep(Duration::from_millis(40));
            post(
                keycode,
                false,
                CGEventFlags::MaskControl | CGEventFlags::MaskAlternate,
            );
            std::thread::sleep(Duration::from_millis(900));

            let launcher = launcher_windows();
            let after = bounds();
            println!(
                "launcher={launcher} moved={} before={before:?} after={after:?}",
                before != after
            );
            dismiss();
            return;
        }

        // Drives the launcher's own create form, which is the only way a record
        // is written by the app rather than by hand.
        // Note for anyone picking this up: driving the launcher's own create
        // form was tried and abandoned. The form is a webview, and synthesized
        // keystrokes reach it out of order even at 180ms spacing ("ProbeName"
        // arriving as "eNamePro"), while a synthesized Tab is typed into the
        // field instead of moving focus. Records written by hand are covered
        // above; a record written by the app is not, and saying so is better
        // than a check that passes for the wrong reason.

        if mode == "type" {
            let text = std::env::args().nth(2).unwrap_or_default();
            std::thread::sleep(Duration::from_secs(1));
            clear_document();
            type_text(&text);
            std::thread::sleep(Duration::from_millis(1500));
            println!("typed {text:?} -> document holds {:?}", document_text());
            return;
        }

        if std::env::args().nth(1).as_deref() == Some("expect-no-expansion") {
            nothing_expands(&mut harness);
            report(&harness);
            return;
        }

        the_hyperkey_carries_its_flags(&mut harness);
        the_lock_state_never_moves(&mut harness);
        a_bound_chord_runs_its_command(&mut harness);
        a_solitary_tap_sends_the_tap_key(&mut harness);
        a_hold_sends_no_tap_key(&mut harness);
        a_chord_sends_no_tap_key(&mut harness);
        a_pause_clears_the_buffer(&mut harness);
        a_keyword_expands_in_place(&mut harness);
        a_keyword_inside_a_word_does_not_expand(&mut harness);

        close_target();
        report(&harness);
    }

    fn report(harness: &Harness) {
        println!(
            "\n{} passed, {} failed",
            harness.passed,
            harness.failed.len()
        );
        for failure in &harness.failed {
            println!("  failed: {failure}");
        }
        if !harness.failed.is_empty() {
            EXIT.store(1, Ordering::SeqCst);
        }
    }

    /// With expansion turned off, a keyword must stay literal. Off has to mean
    /// the monitor is gone, not installed and ignoring what it sees.
    fn nothing_expands(harness: &mut Harness) {
        println!("-- with expansion off, a keyword stays literal");
        if !focus_document(harness, "a keyword stays literal with expansion off") {
            return;
        }
        // A freshly opened document takes a moment before it accepts synthesized
        // keys; without this the typing lands nowhere and the check reads an
        // empty document, which is not the same answer as "it stayed literal".
        std::thread::sleep(Duration::from_secs(1));
        clear_document();
        type_text("hello ;sig");
        std::thread::sleep(Duration::from_millis(1500));
        let text = document_text().unwrap_or_default();
        harness.check(
            "a keyword stays literal with expansion off",
            text.contains(";sig"),
            format!("document holds {text:?}"),
        );
        close_target();
    }

    // -- the checks ---------------------------------------------------------

    /// The claim no application can answer: while the hyperkey is held, the
    /// events passing through carry the four hyper flags.
    fn the_hyperkey_carries_its_flags(harness: &mut Harness) {
        println!("-- the hyperkey adds its flags to an event held under it");
        witnessed().lock().unwrap().clear();
        hold_hyper_and_press(UNBOUND);
        std::thread::sleep(SETTLE);

        let seen = witnessed().lock().unwrap().clone();
        let hyper = CGEventFlags::MaskControl
            | CGEventFlags::MaskAlternate
            | CGEventFlags::MaskShift
            | CGEventFlags::MaskCommand;
        let arrow = seen
            .iter()
            .find(|s| s.keycode == UNBOUND as i64 && s.kind == CGEventType::KeyDown);

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

    /// The whole reason the hyperkey remaps before it taps: using the key must
    /// not toggle the lock state. Once CapsLock is F18 there is no lock event
    /// left to suppress, and this is what says so.
    fn the_lock_state_never_moves(harness: &mut Harness) {
        println!("-- using the hyperkey never moves the lock state");
        let before = lock_is_on();
        for _ in 0..3 {
            hold_hyper_and_press(UNBOUND);
            std::thread::sleep(Duration::from_millis(120));
        }
        post(HYPER, true, CGEventFlags::empty());
        std::thread::sleep(Duration::from_millis(60));
        post(HYPER, false, CGEventFlags::empty());
        std::thread::sleep(SETTLE);

        let after = lock_is_on();
        harness.check(
            "three chords and a tap leave the lock state alone",
            before == after,
            format!("lock was {before}, is {after}"),
        );
    }

    /// The end-to-end claim: the chord reaches the command and the command acts
    /// on the window that was focused, not on Dango.
    fn a_bound_chord_runs_its_command(harness: &mut Harness) {
        println!("-- a bound hyper chord runs its command on the focused window");
        if !frontmost_is(&own_process()) {
            harness.check(
                "hyper+left moves the focused window to the left half",
                false,
                "the scratch window was not frontmost, so nothing was driven".into(),
            );
            return;
        }

        let before = bounds();
        // Put the window somewhere that is definitely not the left half first,
        // so landing there cannot be where it already was.
        place(400.0, 200.0, 700.0, 500.0);
        std::thread::sleep(SETTLE);

        hold_hyper_and_press(LEFT_ARROW);
        std::thread::sleep(SETTLE);

        let after = bounds();
        let screen = work_area();
        // The command cycles 1/2, 2/3, 1/3 on repeats, so the check is that the
        // window is anchored left at one of those widths, not that it is always
        // a half.
        let step = match (after, screen) {
            (Some(after), Some(screen)) => {
                let anchored = (after.0 - screen.0).abs() < 8.0 && after.3 > screen.3 * 0.8;
                anchored
                    .then(|| {
                        [("1/2", 0.5), ("2/3", 2.0 / 3.0), ("1/3", 1.0 / 3.0)]
                            .into_iter()
                            .find(|(_, f)| (after.2 - screen.2 * f).abs() < 8.0)
                            .map(|(name, _)| name)
                    })
                    .flatten()
            }
            _ => None,
        };
        harness.check(
            "hyper+left moves the focused window to a left-anchored step",
            step.is_some(),
            match step {
                Some(step) => format!("landed on the left {step} of the work area"),
                None => format!("before {before:?}, after {after:?}, work area {screen:?}"),
            },
        );
    }

    /// Follows the config rather than assuming it: with a tap key configured a
    /// solitary tap must emit it, and with none configured it must emit nothing.
    fn a_solitary_tap_sends_the_tap_key(harness: &mut Harness) {
        let configured = tap_is_configured();
        println!("-- a quick solitary tap, with tap configured: {configured}");
        witnessed().lock().unwrap().clear();
        post(HYPER, true, CGEventFlags::empty());
        std::thread::sleep(Duration::from_millis(60));
        post(HYPER, false, CGEventFlags::empty());
        std::thread::sleep(SETTLE);

        let seen = witnessed().lock().unwrap().clone();
        let escaped = seen
            .iter()
            .any(|s| s.keycode == ESCAPE && s.kind == CGEventType::KeyDown);
        harness.check(
            if configured {
                "a 60ms solitary tap emits Escape"
            } else {
                "a 60ms solitary tap emits nothing when no tap is configured"
            },
            escaped == configured,
            format!("witnessed {:?}", codes(&seen)),
        );
    }

    /// Whether the running config asks for a tap key at all.
    fn tap_is_configured() -> bool {
        let path = std::env::var("DANGO_CONFIG_DIR").unwrap_or_else(|_| {
            format!(
                "{}/.config/dango",
                std::env::var("HOME").unwrap_or_default()
            )
        });
        std::fs::read_to_string(format!("{path}/config.json"))
            .ok()
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
            .and_then(|config| {
                config
                    .get("hyperkey")
                    .and_then(|h| h.get("tap"))
                    .and_then(|t| t.as_str())
                    .map(|t| !t.trim().is_empty())
            })
            .unwrap_or(false)
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

    /// A chord is not a solitary tap, so it must emit no tap key even though it
    /// is over in well under the threshold.
    fn a_chord_sends_no_tap_key(harness: &mut Harness) {
        println!("-- a quick chord emits no tap key");
        witnessed().lock().unwrap().clear();
        hold_hyper_and_press(UNBOUND);
        std::thread::sleep(SETTLE);

        let seen = witnessed().lock().unwrap().clone();
        harness.check(
            "a chord under the threshold emits no Escape",
            !seen
                .iter()
                .any(|s| s.keycode == ESCAPE && s.kind == CGEventType::KeyDown),
            format!("witnessed {:?}", codes(&seen)),
        );
    }

    /// The buffer is forgetful by design, so a keyword split by a pause longer
    /// than the monitor's must not expand.
    fn a_pause_clears_the_buffer(harness: &mut Harness) {
        println!("-- a pause in the middle of a keyword clears the buffer");
        if !focus_document(harness, "a keyword split by a pause does not expand") {
            return;
        }
        clear_document();
        type_text(";si");
        // Longer than the monitor's own pause, so the buffer is dropped.
        std::thread::sleep(Duration::from_secs(5));
        type_text("g");
        std::thread::sleep(Duration::from_millis(1200));

        let text = document_text().unwrap_or_default();
        harness.check(
            "a keyword split by a pause does not expand",
            text.contains(";sig"),
            format!("document holds {text:?}"),
        );
    }

    fn a_keyword_expands_in_place(harness: &mut Harness) {
        println!("-- typing a keyword replaces it with the snippet");
        if !focus_document(harness, "a keyword typed in a document expands in place") {
            return;
        }
        let text = type_and_read("hello ;sig");
        harness.check(
            "a keyword typed in a document expands in place",
            !text.contains(";sig") && text.len() > "hello ".len(),
            format!("document holds {text:?}"),
        );
    }

    /// `;sig` is punctuation-led, and a punctuation-led keyword is specified to
    /// match even directly after a word. The word-boundary rule is about
    /// keywords that start with a letter, so this uses `brb`.
    fn a_keyword_inside_a_word_does_not_expand(harness: &mut Harness) {
        println!("-- a letter-led keyword inside a longer word does not expand");
        if !focus_document(harness, "a keyword inside a word is left alone") {
            return;
        }
        let inside = type_and_read("abrb");
        harness.check(
            "a keyword inside a word is left alone",
            inside.to_lowercase().contains("abrb"),
            format!("document holds {inside:?}"),
        );

        let after_space = type_and_read("a brb");
        harness.check(
            "the same keyword after a space does expand",
            !after_space.is_empty() && !after_space.to_lowercase().contains("brb"),
            format!("document holds {after_space:?}"),
        );
    }

    fn focus_document(harness: &mut Harness, what: &str) -> bool {
        if frontmost_is(&own_process()) {
            return true;
        }
        raise_target();
        if frontmost_is(&own_process()) {
            return true;
        }
        harness.check(
            what,
            false,
            "the scratch window could not be brought to the front".into(),
        );
        false
    }

    // -- driving keys -------------------------------------------------------

    /// Posts a chord and answers whether the launcher came up.
    fn summons(keycode: u16, flags: CGEventFlags) -> bool {
        post(keycode, true, flags);
        std::thread::sleep(Duration::from_millis(40));
        post(keycode, false, flags);
        std::thread::sleep(Duration::from_millis(900));
        launcher_windows() > 0
    }

    fn dismiss() {
        post(ESCAPE_KEY, true, CGEventFlags::empty());
        post(ESCAPE_KEY, false, CGEventFlags::empty());
        std::thread::sleep(Duration::from_millis(600));
    }

    /// How many windows the launcher process is showing. It is a non-activating
    /// panel, so it never becomes frontmost and the count is the only signal
    /// that it is on screen.
    fn launcher_windows() -> usize {
        osascript(
            "tell application \"System Events\" to tell process \"dango\" to get count of windows",
        )
        .and_then(|text| text.trim().parse().ok())
        .unwrap_or(0)
    }

    /// Whether the CapsLock lock state is currently on, asked of the system
    /// rather than inferred from anything this harness did.
    fn lock_is_on() -> bool {
        CGEventSource::flags_state(CGEventSourceStateID::CombinedSessionState)
            .contains(CGEventFlags::MaskAlphaShift)
    }

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
    //
    // The harness owns its window rather than driving TextEdit. TextEdit cost
    // three separate failures that were nothing to do with Dango: typing lost
    // everything after the first character, an accessibility move reported
    // success while macOS 26 held the window tiled, and document reads came
    // back empty on runs that had worked a minute earlier. A plain window with
    // a plain text view has no document model, no autosave, and no tiling
    // participation, so none of that is in the way.
    //
    // The spike's rule still holds: the process driving the keys is not the one
    // answering. Every read below goes through System Events, which inspects
    // this process from outside, so a keystroke that never arrived cannot be
    // reported as one that did.

    /// Asks System Events to bring this process forward. `NSApplication`'s own
    /// activate does not move an unbundled binary to the front even once it is a
    /// foreground application, and a window that is not key receives no keys.
    fn activate_from_outside() {
        let script = format!(
            "tell application \"System Events\" to tell process \"{}\" to set frontmost to true",
            own_process()
        );
        // Spawned rather than waited on. Waiting would block this process's
        // event loop, and the activation System Events is asking for can only be
        // processed by that loop, so waiting for it prevents it.
        let _ = Command::new("osascript").args(["-e", &script]).spawn();
    }

    /// Promotes this process to a foreground application, so its window can
    /// become key and receive keys.
    fn become_a_foreground_app() {
        #[repr(C)]
        struct ProcessSerialNumber {
            high: u32,
            low: u32,
        }
        const CURRENT_PROCESS: u32 = 2;
        const TO_FOREGROUND: u32 = 1;

        #[link(name = "ApplicationServices", kind = "framework")]
        extern "C" {
            fn TransformProcessType(psn: *const ProcessSerialNumber, transform: u32) -> i32;
        }
        let psn = ProcessSerialNumber {
            high: 0,
            low: CURRENT_PROCESS,
        };
        unsafe { TransformProcessType(&psn, TO_FOREGROUND) };
    }

    /// The process name System Events knows this harness by, which is the
    /// executable's own name.
    fn own_process() -> String {
        std::env::current_exe()
            .ok()
            .and_then(|path| path.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "macos_keys_walkthrough".into())
    }

    /// Builds the scratch window on the main thread and leaves it frontmost and
    /// accepting keys. `Regular` rather than `Accessory`, because a window that
    /// cannot become active cannot receive a synthesized keystroke.
    fn build_window() -> bool {
        let Some(mtm) = MainThreadMarker::new() else {
            eprintln!("the scratch window must be built on the main thread");
            return false;
        };
        // An unbundled binary is not a foreground application, and one that is
        // not cannot be activated, so a synthesized keystroke would never reach
        // its window. This is the Carbon call that promotes it; setting the
        // activation policy alone does not, which is what the earlier failure
        // to come to the front was.
        become_a_foreground_app();

        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
        app.finishLaunching();

        let frame = NSRect::new(NSPoint::new(300.0, 300.0), NSSize::new(700.0, 500.0));
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                frame,
                NSWindowStyleMask::Titled
                    | NSWindowStyleMask::Closable
                    | NSWindowStyleMask::Resizable,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        window.setTitle(&NSString::from_str("dango keys walkthrough"));

        let scroll = NSScrollView::initWithFrame(NSScrollView::alloc(mtm), frame);
        let text = NSTextView::initWithFrame(NSTextView::alloc(mtm), frame);
        {
            text.setRichText(false);
            text.setEditable(true);
            // Every one of these would otherwise rewrite what was typed, and a
            // keyword the target quietly changed is the one failure this check
            // must not confuse for Dango's.
            text.setAutomaticQuoteSubstitutionEnabled(false);
            text.setAutomaticDashSubstitutionEnabled(false);
            text.setAutomaticTextReplacementEnabled(false);
            text.setAutomaticSpellingCorrectionEnabled(false);
            scroll.setDocumentView(Some(&text));
            scroll.setHasVerticalScroller(true);
        }
        window.setContentView(Some(&scroll));
        window.makeKeyAndOrderFront(None);
        window.makeFirstResponder(Some(&text));
        install_edit_menu(mtm, &app);
        app.activate();

        TEXT_VIEW.with(|slot| *slot.borrow_mut() = Some(text));
        WINDOW.with(|slot| *slot.borrow_mut() = Some(window));
        true
    }

    /// A text view answers Cmd+V through its Edit menu's key equivalent, not on
    /// its own, so an application with no menu silently ignores a paste. Without
    /// this the expansion's backspaces land and its paste does not, which looks
    /// exactly like Dango failing to insert.
    fn install_edit_menu(mtm: MainThreadMarker, app: &NSApplication) {
        let main = NSMenu::new(mtm);

        let edit_item = NSMenuItem::new(mtm);
        let edit = NSMenu::new(mtm);
        unsafe {
            edit.setTitle(&NSString::from_str("Edit"));
            for (title, selector, key) in [
                ("Cut", sel!(cut:), "x"),
                ("Copy", sel!(copy:), "c"),
                ("Paste", sel!(paste:), "v"),
                ("Select All", sel!(selectAll:), "a"),
            ] {
                let item = NSMenuItem::initWithTitle_action_keyEquivalent(
                    NSMenuItem::alloc(mtm),
                    &NSString::from_str(title),
                    Some(selector),
                    &NSString::from_str(key),
                );
                edit.addItem(&item);
            }
            edit_item.setSubmenu(Some(&edit));
            main.addItem(&edit_item);
        }
        app.setMainMenu(Some(&main));
    }

    /// Brings the scratch window forward and confirms it from outside. Runs on
    /// the worker, so the main thread stays free to process the activation.
    fn focus_target() -> bool {
        for _ in 0..25 {
            activate_from_outside();
            std::thread::sleep(Duration::from_millis(300));
            if frontmost_is(&own_process()) {
                std::thread::sleep(SETTLE);
                return true;
            }
        }
        eprintln!(
            "expected {:?} to be frontmost, System Events reports {:?}",
            own_process(),
            osascript(
                "tell application \"System Events\" to get name of first application process \
                 whose frontmost is true"
            )
        );
        false
    }

    /// Runs the window's own event loop for a while. The checks run on the main
    /// thread between drives, so the window only processes events when this is
    /// called - which is also what makes a synthesized keystroke land.
    fn pump(duration: Duration) {
        let Some(mtm) = MainThreadMarker::new() else {
            std::thread::sleep(duration);
            return;
        };
        let app = NSApplication::sharedApplication(mtm);
        let deadline = Instant::now() + duration;
        while Instant::now() < deadline {
            let event = unsafe {
                app.nextEventMatchingMask_untilDate_inMode_dequeue(
                    NSEventMask::Any,
                    Some(&NSDate::dateWithTimeIntervalSinceNow(0.01)),
                    NSDefaultRunLoopMode,
                    true,
                )
            };
            match event {
                Some(event) => app.sendEvent(&event),
                None => std::thread::sleep(Duration::from_millis(5)),
            }
        }
    }

    /// Brings the existing scratch window back to the front. The window is
    /// built once; anything after that is activation, not a reopen.
    fn raise_target() {
        focus_target();
    }

    /// Nothing to close: the window goes when the process does, and closing it
    /// from the worker would be touching AppKit off the main thread.
    fn close_target() {}

    /// The spike's lesson: confirm which application is frontmost before
    /// driving it, rather than trusting that the window came up.
    fn frontmost_is(process: &str) -> bool {
        osascript(
            "tell application \"System Events\" to get name of first application process \
             whose frontmost is true",
        )
        .is_some_and(|name| name.trim() == process)
    }

    /// Read from outside this process, so the answer is never this process's
    /// own opinion of what it received.
    /// An empty text view is a real answer, so this keeps the empty string
    /// rather than folding it into "could not read", which is what made an
    /// unarrived keystroke look the same as an empty document.
    fn document_text() -> Option<String> {
        let script = format!(
            "tell application \"System Events\" to tell process \"{}\" to get value of \
             text area 1 of scroll area 1 of window 1",
            own_process()
        );
        let output = Command::new("osascript")
            .args(["-e", &script])
            .output()
            .ok()?;
        output.status.success().then(|| {
            String::from_utf8_lossy(&output.stdout)
                .trim_end()
                .to_string()
        })
    }

    /// Empties the text view. Written through System Events rather than
    /// straight into the `NSTextView`, because this runs on the worker and
    /// AppKit objects belong to the main thread.
    fn clear_document() {
        let _ = osascript(&format!(
            "tell application \"System Events\" to tell process \"{}\" to set value of \
             text area 1 of scroll area 1 of window 1 to \"\"",
            own_process()
        ));
        std::thread::sleep(Duration::from_millis(200));
    }

    /// Types into the scratch window and answers what System Events then reads
    /// back out of it.
    fn type_and_read(text: &str) -> String {
        clear_document();
        type_text(text);
        std::thread::sleep(Duration::from_millis(1600));
        document_text().unwrap_or_default()
    }

    /// Moves the scratch window. Setting the frame on a window this process
    /// owns is the reason the target changed: an accessibility write into
    /// another application reports success while macOS 26 holds a tiled window
    /// exactly where it was.
    fn place(x: f64, y: f64, width: f64, height: f64) {
        let _ = osascript(&format!(
            "tell application \"System Events\" to tell process \"{}\" to set \
             {{value of attribute \"AXPosition\" of window 1, \
               value of attribute \"AXSize\" of window 1}} to \
             {{{{{x}, {y}}}, {{{width}, {height}}}}}",
            own_process()
        ));
        std::thread::sleep(Duration::from_millis(300));
    }

    fn bounds() -> Option<(f64, f64, f64, f64)> {
        let window = "(first window whose subrole is \"AXStandardWindow\")";
        let raw = osascript(&format!(
            "tell application \"System Events\" to tell process \"{}\" to get \
             {{value of attribute \"AXPosition\" of {window}, \
               value of attribute \"AXSize\" of {window}}}",
            own_process()
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
        let raw = osascript("tell application \"Finder\" to get bounds of window of desktop")?;
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
            .args(["-f", "target/release/dango|Dango.app/Contents/MacOS/dango"])
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
        let output = Command::new("osascript")
            .args(["-e", script])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        (!text.is_empty()).then_some(text)
    }
}
