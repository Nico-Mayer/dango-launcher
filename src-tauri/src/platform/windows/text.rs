//! The Windows half of selection and paste.
//!
//! Two things are different from macOS and both are about the foreground.
//! Windows genuinely loses focus to the launcher, so it has to be taken back
//! and, more importantly, waited for: foreground lock can refuse the request,
//! and pasting into whatever won instead is the failure worth avoiding.
//!
//! The second is a borrowed trick. The clipboard watcher already asks the
//! copying application to answer a no-op message before reading, because a
//! window that answers is back in its message loop. The same question pointed
//! the other way says when a paste has been handled, which is when the
//! clipboard is safe to put back. macOS has no equivalent and waits instead.

use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use enigo::{Direction, Enigo, Key, Keyboard as _, Settings};
use windows_sys::Win32::Foundation::{CloseHandle, HWND};
use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, GetForegroundWindow, GetWindowThreadProcessId, SendMessageTimeoutW,
    SMTO_ABORTIFHUNG, WM_NULL,
};

use crate::text::{Handoff, Keys, TextError};

/// Marks every event Dango injects, so M5's key monitor can tell its own
/// output from the user's.
const DANGO_INJECTED: usize = 0x44_41_4E_47;

const FOREGROUND_TIMEOUT: Duration = Duration::from_millis(400);
const FOREGROUND_POLL: Duration = Duration::from_millis(10);
/// Long enough for a busy application to finish handling the paste, short
/// enough that a hung one does not hold the clipboard hostage.
const PASTE_ACK_TIMEOUT_MS: u32 = 500;

/// There is exactly one launcher window in the process, so the window it
/// covered is process-wide rather than threaded through three constructors.
static PREVIOUS_FOREGROUND: AtomicIsize = AtomicIsize::new(0);

pub fn remember_previous_foreground(hwnd: isize) {
    PREVIOUS_FOREGROUND.store(hwnd, Ordering::SeqCst);
}

fn previous_foreground() -> Option<HWND> {
    match PREVIOUS_FOREGROUND.load(Ordering::SeqCst) {
        0 => None,
        hwnd => Some(hwnd as HWND),
    }
}

pub struct WindowsKeys {
    enigo: Mutex<Enigo>,
}

impl WindowsKeys {
    pub fn new() -> Option<Self> {
        let settings = Settings {
            windows_dw_extra_info: Some(DANGO_INJECTED),
            ..Default::default()
        };
        match Enigo::new(&settings) {
            Ok(enigo) => Some(Self {
                enigo: Mutex::new(enigo),
            }),
            Err(error) => {
                eprintln!("[dango] no key injection available: {error}");
                None
            }
        }
    }

    fn chord(&self, letter: char) -> Result<(), TextError> {
        let mut enigo = self.enigo.lock().unwrap();
        let send = |enigo: &mut Enigo| -> Result<(), enigo::InputError> {
            enigo.key(Key::Control, Direction::Press)?;
            let pressed = enigo.key(Key::Unicode(letter), Direction::Click);
            enigo.key(Key::Control, Direction::Release)?;
            pressed
        };
        send(&mut enigo).map_err(|error| TextError::TargetUnavailable(error.to_string()))
    }
}

impl Keys for WindowsKeys {
    fn copy(&self) -> Result<(), TextError> {
        self.chord('c')
    }

    fn paste(&self) -> Result<(), TextError> {
        self.chord('v')
    }

    fn caret_left(&self, times: usize) -> Result<(), TextError> {
        let mut enigo = self.enigo.lock().unwrap();
        for _ in 0..times {
            enigo
                .key(Key::LeftArrow, Direction::Click)
                .map_err(|error| TextError::TargetUnavailable(error.to_string()))?;
        }
        Ok(())
    }

    /// Windows gates none of this. The one thing it does refuse is an elevated
    /// target, which is reported where it happens rather than here.
    fn permitted(&self) -> bool {
        true
    }

    fn request_permission(&self) {}
}

pub struct WindowsHandoff;

impl Handoff for WindowsHandoff {
    fn yield_to_previous(&self) -> Result<(), TextError> {
        let Some(previous) = previous_foreground() else {
            return Err(TextError::NoTarget);
        };
        if is_elevated(previous) {
            return Err(TextError::TargetUnavailable(
                "that window belongs to an elevated program, which Dango cannot reach".into(),
            ));
        }

        unsafe {
            AllowSetForegroundWindow(u32::MAX);
            super::force_foreground(previous);
        }

        // Foreground lock can refuse the request outright, so the system is
        // asked who is actually in front rather than being given time and
        // trusted. A keystroke sent to the wrong window is worse than none.
        let deadline = Instant::now() + FOREGROUND_TIMEOUT;
        loop {
            if unsafe { GetForegroundWindow() } == previous {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(TextError::TargetUnavailable(
                    "the previous window would not come back to the foreground".into(),
                ));
            }
            std::thread::sleep(FOREGROUND_POLL);
        }
    }

    /// A window that answers a no-op message is back in its message loop, which
    /// means it has finished handling what it was sent.
    fn settle_after_paste(&self) {
        let Some(target) = previous_foreground() else {
            return;
        };
        let mut result: usize = 0;
        unsafe {
            SendMessageTimeoutW(
                target,
                WM_NULL,
                0,
                0,
                SMTO_ABORTIFHUNG,
                PASTE_ACK_TIMEOUT_MS,
                &mut result,
            );
        }
    }
}

/// A non-elevated process cannot open an elevated one, so the refusal is the
/// answer. This is a hard ceiling of running unelevated, not something to work
/// around.
fn is_elevated(hwnd: HWND) -> bool {
    unsafe {
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            return false;
        }
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return true;
        }
        CloseHandle(handle);
        false
    }
}
