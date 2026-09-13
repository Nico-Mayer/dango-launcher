//! The `CGEventTap` both key services are built on.
//!
//! A tap delivers its callback on whichever run loop its source is added to, so
//! each tap gets a dedicated thread running its own `CFRunLoop`. That keeps the
//! input path off the main thread, which the launcher needs for AppKit, and it
//! gives the handle something to stop: dropping it stops the run loop, which
//! ends the thread, which invalidates the tap.
//!
//! Unlike a Windows low-level hook, a tap gets a user pointer, so nothing here
//! needs process-global state. The context lives on the tap thread's stack for
//! exactly as long as the run loop runs.
//!
//! The system can disable a tap without telling anyone
//! (`TapDisabledByTimeout`, `TapDisabledByUserInput`). It is re-enabled here
//! rather than in either caller, because a tap that silently stops is the way
//! both of these features would most likely fail in the field.

use std::cell::OnceCell;
use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;

use objc2_core_foundation::{kCFRunLoopCommonModes, CFMachPort, CFRetained, CFRunLoop};
use objc2_core_graphics::{
    CGEvent, CGEventMask, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
    CGEventTapProxy, CGEventType,
};

/// What the tap does with an event: pass it on as it stands, pass on a modified
/// event, or swallow it so nothing downstream sees it.
pub(super) enum Verdict {
    Pass,
    Swallow,
}

type Handler = Box<dyn Fn(CGEventType, &CGEvent) -> Verdict + Send>;

struct Context {
    handler: Handler,
    /// Filled immediately after the tap is created and only read from the
    /// callback, which cannot run before the run loop does.
    tap: OnceCell<CFRetained<CFMachPort>>,
}

/// A running tap. Dropping it stops the tap and joins its thread.
pub(super) struct EventTap {
    run_loop: usize,
    thread: Option<JoinHandle<()>>,
}

impl EventTap {
    /// `None` when the system refused the tap, which in practice always means
    /// the Accessibility permission is missing.
    pub(super) fn start(
        options: CGEventTapOptions,
        mask: CGEventMask,
        handler: Handler,
    ) -> Option<Self> {
        let (report, done) = std::sync::mpsc::channel::<Option<usize>>();
        let thread = std::thread::Builder::new()
            .name("dango-eventtap".into())
            .spawn(move || tap_thread(options, mask, handler, report))
            .ok()?;

        match done.recv() {
            Ok(Some(run_loop)) => Some(Self {
                run_loop,
                thread: Some(thread),
            }),
            _ => {
                let _ = thread.join();
                None
            }
        }
    }
}

impl Drop for EventTap {
    fn drop(&mut self) {
        // CFRunLoopStop and CFRunLoopWakeUp are the two run-loop calls
        // documented as safe from another thread. The thread holds a retain
        // until it exits and is joined here, so the pointer is live throughout.
        let run_loop = self.run_loop as *const CFRunLoop;
        unsafe {
            (*run_loop).stop();
            (*run_loop).wake_up();
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn tap_thread(
    options: CGEventTapOptions,
    mask: CGEventMask,
    handler: Handler,
    report: Sender<Option<usize>>,
) {
    let context = Box::new(Context {
        handler,
        tap: OnceCell::new(),
    });
    let user_info = &*context as *const Context as *mut c_void;

    let tap = unsafe {
        CGEvent::tap_create(
            CGEventTapLocation::SessionEventTap,
            CGEventTapPlacement::HeadInsertEventTap,
            options,
            mask,
            Some(trampoline),
            user_info,
        )
    };
    let Some(tap) = tap else {
        let _ = report.send(None);
        return;
    };
    let _ = context.tap.set(tap.clone());

    let Some(source) = CFMachPort::new_run_loop_source(None, Some(&tap), 0) else {
        let _ = report.send(None);
        return;
    };
    let Some(run_loop) = CFRunLoop::current() else {
        let _ = report.send(None);
        return;
    };
    unsafe { run_loop.add_source(Some(&source), kCFRunLoopCommonModes) };
    CGEvent::tap_enable(&tap, true);

    let address = CFRetained::as_ptr(&run_loop).as_ptr() as usize;
    if report.send(Some(address)).is_err() {
        return;
    }

    CFRunLoop::run();

    unsafe { run_loop.remove_source(Some(&source), kCFRunLoopCommonModes) };
    CGEvent::tap_enable(&tap, false);
    source.invalidate();
    tap.invalidate();
}

unsafe extern "C-unwind" fn trampoline(
    _proxy: CGEventTapProxy,
    kind: CGEventType,
    event: NonNull<CGEvent>,
    user_info: *mut c_void,
) -> *mut CGEvent {
    let context = unsafe { &*(user_info as *const Context) };

    if kind == CGEventType::TapDisabledByTimeout || kind == CGEventType::TapDisabledByUserInput {
        if let Some(tap) = context.tap.get() {
            CGEvent::tap_enable(tap, true);
        }
        return event.as_ptr();
    }

    match (context.handler)(kind, unsafe { event.as_ref() }) {
        Verdict::Pass => event.as_ptr(),
        Verdict::Swallow => std::ptr::null_mut(),
    }
}

/// The bit in a `CGEventMask` for one event type.
pub(super) const fn mask_bit(kind: CGEventType) -> CGEventMask {
    1 << kind.0
}
