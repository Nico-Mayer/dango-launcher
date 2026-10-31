//! M5 spike, macOS half. The questions the hyperkey and key monitor designs
//! need answered before either is written:
//!
//! 1. Which event does CapsLock arrive as at a session tap, and with what
//!    keycode and flags.
//! 2. Whether returning null for it suppresses the CapsLock toggle, or whether
//!    the HID system has already toggled by the time a session tap sees it.
//! 3. Whether an `hidutil` remap of CapsLock turns it into an ordinary key
//!    event that a tap can own outright.
//! 4. What `keyboard_get_unicode_string` reports for ordinary typing.
//! 5. Whether events posted by this process (the shape enigo posts) are
//!    distinguishable from the user's own at the tap.
//!
//! Usage:
//!   `cargo run --example keytap_spike -- watch`      1, 4, 5: log every event
//!   `cargo run --example keytap_spike -- suppress`   2: swallow CapsLock
//!   `cargo run --example keytap_spike -- post`       drive keys from here

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("keytap_spike only runs on macOS");
}

#[cfg(target_os = "macos")]
fn main() {
    spike::run();
}

#[cfg(target_os = "macos")]
mod spike {
    use std::ffi::c_void;
    use std::ptr::NonNull;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::time::{Duration, Instant};

    use objc2_core_foundation::{kCFRunLoopCommonModes, CFMachPort, CFRunLoop};
    use objc2_core_graphics::{
        CGEvent, CGEventField, CGEventFlags, CGEventSource, CGEventSourceStateID,
        CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventTapProxy, CGEventType,
    };

    static SUPPRESS_CAPS: AtomicBool = AtomicBool::new(false);
    static SEEN: AtomicU64 = AtomicU64::new(0);
    static TOTAL_NANOS: AtomicU64 = AtomicU64::new(0);

    pub fn run() {
        let mode = std::env::args().nth(1).unwrap_or_else(|| "watch".into());
        match mode.as_str() {
            "suppress" => {
                SUPPRESS_CAPS.store(true, Ordering::SeqCst);
                watch();
            }
            "post" => post(),
            _ => watch(),
        }
    }

    fn watch() {
        println!("[spike] accessibility trusted: {}", trusted());
        let mask = (1u64 << CGEventType::KeyDown.0)
            | (1u64 << CGEventType::KeyUp.0)
            | (1u64 << CGEventType::FlagsChanged.0);
        let tap = unsafe {
            CGEvent::tap_create(
                CGEventTapLocation::SessionEventTap,
                CGEventTapPlacement::HeadInsertEventTap,
                CGEventTapOptions::Default,
                mask,
                Some(callback),
                std::ptr::null_mut(),
            )
        };
        let Some(tap) = tap else {
            eprintln!("[spike] tap_create returned null: no Accessibility permission");
            return;
        };
        let source = CFMachPort::new_run_loop_source(None, Some(&tap), 0).unwrap();
        let run_loop = CFRunLoop::current().unwrap();
        unsafe { run_loop.add_source(Some(&source), kCFRunLoopCommonModes) };
        CGEvent::tap_enable(&tap, true);
        println!(
            "[spike] tap running (suppress_caps={}). Ctrl-C to stop.",
            SUPPRESS_CAPS.load(Ordering::SeqCst)
        );
        CFRunLoop::run();
    }

    unsafe extern "C-unwind" fn callback(
        _proxy: CGEventTapProxy,
        kind: CGEventType,
        event: NonNull<CGEvent>,
        _user: *mut c_void,
    ) -> *mut CGEvent {
        let start = Instant::now();
        let event_ref = unsafe { event.as_ref() };

        if kind == CGEventType::TapDisabledByTimeout || kind == CGEventType::TapDisabledByUserInput
        {
            println!("[spike] !!! tap disabled by the system: {kind:?}");
            return event.as_ptr();
        }

        let keycode =
            CGEvent::integer_value_field(Some(event_ref), CGEventField::KeyboardEventKeycode);
        let autorepeat =
            CGEvent::integer_value_field(Some(event_ref), CGEventField::KeyboardEventAutorepeat);
        let user_data =
            CGEvent::integer_value_field(Some(event_ref), CGEventField::EventSourceUserData);
        let state_id =
            CGEvent::integer_value_field(Some(event_ref), CGEventField::EventSourceStateID);
        let flags = CGEvent::flags(Some(event_ref));
        let text = unicode_string(event_ref);

        println!(
            "[spike] type={:<12} keycode={keycode:<4} repeat={autorepeat} flags={:#x} \
             state_id={state_id} user_data={user_data} text={text:?}",
            format!("{:?}", kind),
            flags.0
        );

        let elapsed = start.elapsed();
        SEEN.fetch_add(1, Ordering::Relaxed);
        TOTAL_NANOS.fetch_add(elapsed.as_nanos() as u64, Ordering::Relaxed);
        let seen = SEEN.load(Ordering::Relaxed);
        if seen.is_multiple_of(10) {
            println!(
                "[spike] mean callback cost over {seen}: {:.2}µs",
                TOTAL_NANOS.load(Ordering::Relaxed) as f64 / seen as f64 / 1000.0
            );
        }

        // 2: swallow CapsLock and see whether the toggle still happens.
        if SUPPRESS_CAPS.load(Ordering::SeqCst) && keycode == 57 {
            println!("[spike]   -> swallowed");
            return std::ptr::null_mut();
        }
        event.as_ptr()
    }

    fn unicode_string(event: &CGEvent) -> String {
        let mut buffer = [0u16; 8];
        let mut len: u64 = 0;
        unsafe {
            CGEvent::keyboard_get_unicode_string(
                Some(event),
                buffer.len() as u64,
                &mut len,
                buffer.as_mut_ptr(),
            );
        }
        String::from_utf16_lossy(&buffer[..(len as usize).min(buffer.len())])
    }

    /// 3 and 5: drive keys the way enigo does, so the watching instance can say
    /// whether they are distinguishable from the user's own.
    fn post() {
        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState);
        println!("[spike] posting in 3s: a, then CapsLock down/up");
        std::thread::sleep(Duration::from_secs(3));

        for down in [true, false] {
            let event = CGEvent::new_keyboard_event(source.as_deref(), 0, down).unwrap();
            CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&event));
            std::thread::sleep(Duration::from_millis(30));
        }
        std::thread::sleep(Duration::from_millis(300));
        for flags in [CGEventFlags::MaskAlphaShift, CGEventFlags::empty()] {
            let event =
                CGEvent::new_keyboard_event(source.as_deref(), 57, !flags.is_empty()).unwrap();
            CGEvent::set_type(Some(&event), CGEventType::FlagsChanged);
            CGEvent::set_flags(Some(&event), flags);
            CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&event));
            std::thread::sleep(Duration::from_millis(60));
        }
        println!("[spike] posted");
    }

    fn trusted() -> bool {
        unsafe { objc2_application_services::AXIsProcessTrusted() }
    }
}
