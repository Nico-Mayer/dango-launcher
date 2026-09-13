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

/// The window an insertion goes to. The launcher records it on show; the
/// walkthrough harness sets it to a window of its own.
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
        if is_out_of_reach(previous) {
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
fn is_out_of_reach(hwnd: HWND) -> bool {
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
