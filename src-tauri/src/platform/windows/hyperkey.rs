//! The Windows hyperkey: a low-level keyboard hook that turns one physical key
//! into the hyper modifier.
//!
//! While the mapped key is held, the hook holds the configured modifiers down
//! (Ctrl+Alt+Super, plus Shift unless excluded) and swallows the key's own
//! function, so a `hyper+X` chord fires through the same global-shortcut path as
//! any other chord. The hook's own injected modifiers carry `DANGO_INJECTED`,
//! the marker the paste path already stamps, and are passed straight through so
//! they set real modifier state and never loop back into the hook.
//!
//! A low-level hook gets no user pointer, so its callback reads process-global
//! state. There is at most one hyperkey installed at a time: a reload drops the
//! old handle, which joins its thread and releases any held modifiers, before
//! starting a new one.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;

use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_CAPITAL,
    VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx,
    HC_ACTION, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN,
    WM_SYSKEYUP,
};

use super::DANGO_INJECTED;
use crate::platform::{Hyperkey, HyperkeySpec, HyperkeyTrigger};

static TRIGGER_VK: AtomicU32 = AtomicU32::new(0);
static EMIT_CTRL: AtomicBool = AtomicBool::new(false);
static EMIT_ALT: AtomicBool = AtomicBool::new(false);
static EMIT_SHIFT: AtomicBool = AtomicBool::new(false);
static EMIT_META: AtomicBool = AtomicBool::new(false);
static HELD: AtomicBool = AtomicBool::new(false);

pub struct WindowsHyperkey {
    thread_id: u32,
    thread: Option<JoinHandle<()>>,
}

impl WindowsHyperkey {
    pub fn start(spec: HyperkeySpec) -> Option<Self> {
        let vk = match spec.trigger {
            HyperkeyTrigger::CapsLock => VK_CAPITAL as u32,
        };
        EMIT_CTRL.store(spec.emit.ctrl, Ordering::SeqCst);
        EMIT_ALT.store(spec.emit.alt, Ordering::SeqCst);
        EMIT_SHIFT.store(spec.emit.shift, Ordering::SeqCst);
        EMIT_META.store(spec.emit.meta, Ordering::SeqCst);
        HELD.store(false, Ordering::SeqCst);
        TRIGGER_VK.store(vk, Ordering::SeqCst);

        let (report, done) = std::sync::mpsc::channel::<Option<u32>>();
        let thread = std::thread::Builder::new()
            .name("dango-hyperkey".into())
            .spawn(move || hook_thread(report))
            .ok()?;

        match done.recv() {
            Ok(Some(thread_id)) => Some(Self {
                thread_id,
                thread: Some(thread),
            }),
            _ => {
                TRIGGER_VK.store(0, Ordering::SeqCst);
                let _ = thread.join();
                None
            }
        }
    }
}

impl Hyperkey for WindowsHyperkey {}

impl Drop for WindowsHyperkey {
    fn drop(&mut self) {
        unsafe {
            PostThreadMessageW(self.thread_id, WM_QUIT, 0, 0);
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn hook_thread(report: Sender<Option<u32>>) {
    unsafe {
        let hmod = GetModuleHandleW(std::ptr::null());
        let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), hmod, 0);
        if hook.is_null() {
            let _ = report.send(None);
            return;
        }
        let _ = report.send(Some(GetCurrentThreadId()));

        // A low-level hook needs a message loop on its own thread. GetMessage
        // returns 0 on the WM_QUIT that Drop posts, which ends the loop.
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {}

        // Release anything still held so no modifier is left stuck down, then
        // remove the hook.
        if HELD.swap(false, Ordering::SeqCst) {
            emit_modifiers(false);
        }
        UnhookWindowsHookEx(hook);
        TRIGGER_VK.store(0, Ordering::SeqCst);
    }
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // The first argument of CallNextHookEx is ignored on current Windows.
    if code != HC_ACTION as i32 {
        return CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam);
    }
    let info = &*(lparam as *const KBDLLHOOKSTRUCT);

    if info.dwExtraInfo == DANGO_INJECTED {
        return CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam);
    }

    let trigger = TRIGGER_VK.load(Ordering::SeqCst);
    if trigger != 0 && info.vkCode == trigger {
        let edge = match wparam as u32 {
            WM_KEYDOWN | WM_SYSKEYDOWN => Some(KeyEdge::Down),
            WM_KEYUP | WM_SYSKEYUP => Some(KeyEdge::Up),
            _ => None,
        };
        if let Some(edge) = edge {
            // The hook callback runs only on the thread that installed it, so
            // this load/store pair never races another thread.
            let (held, emit) = transition(HELD.load(Ordering::SeqCst), edge);
            HELD.store(held, Ordering::SeqCst);
            if emit {
                emit_modifiers(matches!(edge, KeyEdge::Down));
            }
            return 1; // swallow the key, so CapsLock never toggles
        }
    }

    CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
}

#[derive(Clone, Copy)]
enum KeyEdge {
    Down,
    Up,
}

/// The held-state transition, kept pure so it can be tested. Given whether the
/// hyperkey is currently held and the edge seen, returns the new held state and
/// whether to press or release the modifiers: only the first down presses (an
/// auto-repeat down does not), and only an up while held releases.
fn transition(held: bool, edge: KeyEdge) -> (bool, bool) {
    match edge {
        KeyEdge::Down => (true, !held),
        KeyEdge::Up => (false, held),
    }
}

/// Presses or releases the configured modifiers. Win goes down first and up last
/// so it is never pressed and released "clean": the Ctrl and Alt events around
/// it stop Windows from opening the Start menu on a lone hyperkey tap.
unsafe fn emit_modifiers(down: bool) {
    let meta = EMIT_META.load(Ordering::SeqCst);
    let ctrl = EMIT_CTRL.load(Ordering::SeqCst);
    let alt = EMIT_ALT.load(Ordering::SeqCst);
    let shift = EMIT_SHIFT.load(Ordering::SeqCst);
    if down {
        if meta {
            send_key(VK_LWIN, true);
        }
        if ctrl {
            send_key(VK_LCONTROL, true);
        }
        if alt {
            send_key(VK_LMENU, true);
        }
        if shift {
            send_key(VK_LSHIFT, true);
        }
    } else {
        if shift {
            send_key(VK_LSHIFT, false);
        }
        if alt {
            send_key(VK_LMENU, false);
        }
        if ctrl {
            send_key(VK_LCONTROL, false);
        }
        if meta {
            send_key(VK_LWIN, false);
        }
    }
}

unsafe fn send_key(vk: VIRTUAL_KEY, down: bool) {
    let mut input: INPUT = std::mem::zeroed();
    input.r#type = INPUT_KEYBOARD;
    input.Anonymous.ki = KEYBDINPUT {
        wVk: vk,
        wScan: 0,
        dwFlags: if down { 0 } else { KEYEVENTF_KEYUP },
        time: 0,
        dwExtraInfo: DANGO_INJECTED,
    };
    SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
}

#[cfg(test)]
mod tests {
    use super::{transition, KeyEdge};

    #[test]
    fn first_down_presses_and_repeats_do_not() {
        let (held, emit) = transition(false, KeyEdge::Down);
        assert!(held && emit, "the first down presses the modifiers");
        let (held, emit) = transition(true, KeyEdge::Down);
        assert!(held && !emit, "an auto-repeat down does not press again");
    }

    #[test]
    fn up_releases_only_while_held() {
        let (held, emit) = transition(true, KeyEdge::Up);
        assert!(!held && emit, "an up while held releases the modifiers");
        let (held, emit) = transition(false, KeyEdge::Up);
        assert!(!held && !emit, "an up while not held releases nothing");
    }
}
