use std::sync::mpsc::Sender;

use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetMessageW, SetTimer, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, MSG, WH_KEYBOARD_LL,
    WM_TIMER,
};

pub(super) type HookProc = unsafe extern "system" fn(i32, WPARAM, LPARAM) -> LRESULT;

// Windows silently removes a low-level hook whose procedure exceeds
// `LowLevelHooksTimeout` and never says so, so the hook is replaced on a timer
// instead of waiting for a removal that cannot be observed.
const REINSTALL_EVERY_MS: u32 = 5_000;

/// Installs `proc` as a `WH_KEYBOARD_LL` hook and pumps this thread's messages
/// until `WM_QUIT`. Reports the thread id once installed, or `None` if it could
/// not be.
pub(super) fn run(proc: HookProc, report: Sender<Option<u32>>) {
    unsafe {
        let Some(mut hook) = install(proc) else {
            let _ = report.send(None);
            return;
        };
        let _ = report.send(Some(GetCurrentThreadId()));
        SetTimer(std::ptr::null_mut(), 0, REINSTALL_EVERY_MS, None);

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            // Hook callbacks run only while this thread pumps messages, so no
            // event reaches both hooks between installing and removing.
            if msg.message == WM_TIMER {
                if let Some(fresh) = install(proc) {
                    UnhookWindowsHookEx(hook);
                    hook = fresh;
                }
            }
        }

        UnhookWindowsHookEx(hook);
    }
}

unsafe fn install(proc: HookProc) -> Option<HHOOK> {
    let hook = SetWindowsHookExW(
        WH_KEYBOARD_LL,
        Some(proc),
        GetModuleHandleW(std::ptr::null()),
        0,
    );
    (!hook.is_null()).then_some(hook)
}
