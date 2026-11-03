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
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, HWND};
use windows_sys::Win32::Security::{
    GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, TokenIntegrityLevel,
    TOKEN_MANDATORY_LABEL, TOKEN_QUERY,
};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, GetForegroundWindow, GetWindowThreadProcessId, IsWindowVisible,
    SendMessageTimeoutW, SMTO_ABORTIFHUNG, WM_NULL,
};

use super::DANGO_INJECTED;
use crate::text::{Handoff, Keys, Lend, TextError};

const FOREGROUND_TIMEOUT: Duration = Duration::from_millis(400);
const FOREGROUND_POLL: Duration = Duration::from_millis(10);
/// New Notepad can handle a key well after it was sent. Released at once,
/// Ctrl is already up when it reads the V, and it types "v" instead of pasting.
const CHORD_HOLD: Duration = Duration::from_millis(30);
/// Long enough for a busy application to finish handling the paste, short
/// enough that a hung one does not hold the clipboard hostage.
const PASTE_ACK_TIMEOUT_MS: u32 = 500;

/// There is exactly one launcher window in the process, so the window it
/// covered is process-wide rather than threaded through three constructors.
static PREVIOUS_FOREGROUND: AtomicIsize = AtomicIsize::new(0);

/// The window an insertion goes to. The launcher records it on show; the
/// walkthrough harness sets it to a window of its own.
pub fn remember_previous_foreground(hwnd: isize) {
    PREVIOUS_FOREGROUND.store(hwnd, Ordering::SeqCst);
}

pub(super) fn previous_foreground() -> Option<HWND> {
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

    fn chord(&self, key: Key) -> Result<(), TextError> {
        let mut enigo = self.enigo.lock().unwrap();
        let send = |enigo: &mut Enigo| -> Result<(), enigo::InputError> {
            enigo.key(Key::Control, Direction::Press)?;
            let pressed = enigo.key(key, Direction::Click);
            std::thread::sleep(CHORD_HOLD);
            enigo.key(Key::Control, Direction::Release)?;
            pressed
        };
        send(&mut enigo).map_err(keystroke_failed)
    }
}

fn keystroke_failed(error: enigo::InputError) -> TextError {
    eprintln!("[dango] keystroke failed: {error}");
    TextError::TargetUnavailable(crate::text::KEYSTROKE_FAILED.into())
}

impl Keys for WindowsKeys {
    /// Ctrl+C is SIGINT in terminals and a command in modal editors.
    /// Ctrl+Insert means copy there too, and nothing else anywhere.
    fn copy(&self) -> Result<(), TextError> {
        self.chord(Key::Insert)
    }

    fn paste(&self) -> Result<(), TextError> {
        self.chord(Key::Unicode('v'))
    }

    fn caret_left(&self, times: usize) -> Result<(), TextError> {
        let mut enigo = self.enigo.lock().unwrap();
        for _ in 0..times {
            enigo
                .key(Key::LeftArrow, Direction::Click)
                .map_err(keystroke_failed)?;
        }
        Ok(())
    }

    fn backspace(&self, times: usize) -> Result<(), TextError> {
        let mut enigo = self.enigo.lock().unwrap();
        for _ in 0..times {
            enigo
                .key(Key::Backspace, Direction::Click)
                .map_err(keystroke_failed)?;
        }
        Ok(())
    }

    /// Windows gates none of this. The one thing it does refuse is an elevated
    /// target, which is reported where it happens rather than here.
    fn permitted(&self) -> bool {
        true
    }
}

pub struct WindowsHandoff;

impl Handoff for WindowsHandoff {
    fn yield_to_previous(&self) -> Result<(), TextError> {
        let Some(previous) = previous_foreground() else {
            return Err(TextError::NoTarget);
        };
        if is_out_of_reach(previous) {
            return Err(TextError::TargetUnavailable(
                "Dango can't paste into administrator apps".into(),
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
                    "The previous app didn't come back to the front".into(),
                ));
            }
            std::thread::sleep(FOREGROUND_POLL);
        }
    }

    /// Gives the foreground back only if Dango held it. Keyword expansion reads
    /// with the launcher hidden, and activating a hidden window would take the
    /// keyboard from the app the user is typing in.
    fn lend_to_previous(&self) -> Lend {
        let launcher = unsafe { GetForegroundWindow() } as isize;
        if launcher == 0 || !super::is_own_window(launcher as HWND) {
            return Lend::new(self.yield_to_previous(), || {});
        }
        super::LENDING.store(true, Ordering::SeqCst);
        Lend::new(self.yield_to_previous(), move || unsafe {
            let launcher = launcher as HWND;
            // Hidden during the read: activating it would leave the keyboard
            // in a window nobody can see.
            if IsWindowVisible(launcher) == 0 {
                super::LENDING.store(false, Ordering::SeqCst);
                return;
            }
            super::force_foreground(launcher);
            if GetForegroundWindow() != launcher {
                eprintln!("[dango] the launcher didn't get the foreground back after a read");
            }
            super::LENDING.store(false, Ordering::SeqCst);
        })
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

/// Whether the target window is out of reach because it outranks this process.
///
/// The ceiling is UIPI: synthesised input from a lower integrity level is
/// silently dropped by a higher-integrity window, so the paste would vanish
/// with no error. The answer is to compare integrity levels and refuse when the
/// target sits above us.
///
/// An earlier version probed with `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)`
/// and read a refusal as "elevated". That was wrong and only showed at runtime:
/// that access right is granted across integrity levels by design, so the open
/// always succeeded and every elevated window read as reachable.
pub(super) fn is_out_of_reach(hwnd: HWND) -> bool {
    unsafe {
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            return false;
        }
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            // Cannot even be opened, so it certainly cannot be driven.
            return true;
        }
        let target = integrity_level(handle);
        CloseHandle(handle);
        match (target, own_integrity_level()) {
            (Some(target), Some(own)) => target > own,
            // Its integrity could not be read while ours could, which a
            // same-or-lower process would not do. Treat it as above us.
            (None, Some(_)) => true,
            _ => false,
        }
    }
}

/// The integrity level of this process, as the last sub-authority of the
/// token's mandatory label: 0x1000 low, 0x2000 medium, 0x3000 high.
pub fn own_integrity_level() -> Option<u32> {
    unsafe { integrity_level(GetCurrentProcess()) }
}

unsafe fn integrity_level(process: HANDLE) -> Option<u32> {
    let mut token: HANDLE = std::ptr::null_mut();
    if OpenProcessToken(process, TOKEN_QUERY, &mut token) == 0 {
        return None;
    }
    // Aligned for the label struct that leads the buffer; the SID follows it.
    let mut buffer = [0u64; 8];
    let mut needed = 0u32;
    let read = GetTokenInformation(
        token,
        TokenIntegrityLevel,
        buffer.as_mut_ptr().cast(),
        std::mem::size_of_val(&buffer) as u32,
        &mut needed,
    );
    CloseHandle(token);
    if read == 0 {
        return None;
    }
    let label = &*(buffer.as_ptr() as *const TOKEN_MANDATORY_LABEL);
    let sid = label.Label.Sid;
    let count = *GetSidSubAuthorityCount(sid);
    Some(*GetSidSubAuthority(sid, u32::from(count) - 1))
}
