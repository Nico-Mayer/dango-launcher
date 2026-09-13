//! The Windows key monitor: a listening `WH_KEYBOARD_LL` hook that reports the
//! character each keystroke produced, for keyword expansion.
//!
//! It reuses the hook-thread pattern the hyperkey built, but it never swallows a
//! key: every event is passed on. For each key-down it works out the character
//! with `ToUnicodeEx` and hands a `KeyStroke` to the sink; a key that is not a
//! plain character, or a chord with a modifier other than Shift, is reported as
//! `Clear`. Its own injected backspaces and paste carry `DANGO_INJECTED` and are
//! ignored, so the monitor never observes Dango's own output.
//!
//! `ToUnicodeEx` is called with the flag that leaves the kernel keyboard state
//! untouched, so translating here does not clobber the dead-key composition the
//! user's real typing depends on.

use std::sync::mpsc::Sender;
use std::sync::Mutex;
use std::thread::JoinHandle;

use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyState, GetKeyboardLayout, ToUnicodeEx, HKL, VK_CAPITAL, VK_CONTROL,
    VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetForegroundWindow, GetMessageW, GetWindowThreadProcessId, PostThreadMessageW,
    SetWindowsHookExW, UnhookWindowsHookEx, HC_ACTION, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL,
    WM_KEYDOWN, WM_QUIT, WM_SYSKEYDOWN,
};

use super::DANGO_INJECTED;
use crate::platform::{KeyMonitor, KeyStroke};

type Sink = Box<dyn Fn(KeyStroke) + Send>;

/// The one running monitor's sink. A low-level hook callback gets no user
/// pointer, so it reads this. The `Mutex` makes it shareable even though the sink
/// itself need only be `Send`.
static SINK: Mutex<Option<Sink>> = Mutex::new(None);

/// Leaves the kernel keyboard state untouched, so translating a keystroke here
/// does not consume a dead key the user's real typing is composing.
const TOUNICODE_NO_STATE_CHANGE: u32 = 0x4;

pub struct WindowsKeyMonitor {
    thread_id: u32,
    thread: Option<JoinHandle<()>>,
}

impl WindowsKeyMonitor {
    pub fn start(sink: Sink) -> Option<Self> {
        *SINK.lock().unwrap() = Some(sink);

        let (report, done) = std::sync::mpsc::channel::<Option<u32>>();
        let thread = std::thread::Builder::new()
            .name("dango-keymonitor".into())
            .spawn(move || hook_thread(report))
            .ok()?;

        match done.recv() {
            Ok(Some(thread_id)) => Some(Self {
                thread_id,
                thread: Some(thread),
            }),
            _ => {
                *SINK.lock().unwrap() = None;
                let _ = thread.join();
                None
            }
        }
    }
}

impl KeyMonitor for WindowsKeyMonitor {}

impl Drop for WindowsKeyMonitor {
    fn drop(&mut self) {
        unsafe {
            PostThreadMessageW(self.thread_id, WM_QUIT, 0, 0);
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        *SINK.lock().unwrap() = None;
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

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {}

        UnhookWindowsHookEx(hook);
    }
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let info = &*(lparam as *const KBDLLHOOKSTRUCT);
        // Never observe Dango's own injected backspaces or paste.
        if info.dwExtraInfo != DANGO_INJECTED {
            let message = wparam as u32;
            if message == WM_KEYDOWN || message == WM_SYSKEYDOWN {
                deliver(translate(info.vkCode, info.scanCode, message));
            }
        }
    }
    // A monitor never swallows a key: always pass the event on.
    CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
}

fn deliver(stroke: KeyStroke) {
    if let Ok(sink) = SINK.lock() {
        if let Some(sink) = sink.as_ref() {
            sink(stroke);
        }
    }
}

unsafe fn down(vk: u16) -> bool {
    (GetAsyncKeyState(vk as i32) as u16 & 0x8000) != 0
}

fn translate(vk_code: u32, scan_code: u32, message: u32) -> KeyStroke {
    unsafe {
        // A chord with anything but Shift is not text; it clears the buffer.
        if message == WM_SYSKEYDOWN
            || down(VK_CONTROL)
            || down(VK_MENU)
            || down(VK_LWIN)
            || down(VK_RWIN)
        {
            return KeyStroke::Clear;
        }

        let layout = current_layout();
        let mut state = [0u8; 256];
        if down(VK_SHIFT) {
            state[VK_SHIFT as usize] = 0x80;
        }
        if GetKeyState(VK_CAPITAL as i32) & 1 != 0 {
            state[VK_CAPITAL as usize] = 1;
        }

        let mut buffer = [0u16; 8];
        let produced = ToUnicodeEx(
            vk_code,
            scan_code,
            state.as_ptr(),
            buffer.as_mut_ptr(),
            buffer.len() as i32,
            TOUNICODE_NO_STATE_CHANGE,
            layout,
        );

        // Exactly one unit, a non-control character, is text. Zero (a function
        // key), a negative (a dead key), or a control character clears.
        if produced == 1 {
            match char::from_u32(buffer[0] as u32) {
                Some(c) if !c.is_control() => KeyStroke::Char(c),
                _ => KeyStroke::Clear,
            }
        } else {
            KeyStroke::Clear
        }
    }
}

unsafe fn current_layout() -> HKL {
    let foreground = GetForegroundWindow();
    let thread = if foreground.is_null() {
        0
    } else {
        GetWindowThreadProcessId(foreground, std::ptr::null_mut())
    };
    GetKeyboardLayout(thread)
}
