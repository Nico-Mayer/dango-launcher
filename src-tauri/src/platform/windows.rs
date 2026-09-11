use std::ffi::c_void;

use tauri::WebviewWindow;
use windows_sys::Win32::Foundation::{HWND, TRUE};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, AttachThreadInput, GetForegroundWindow, GetWindowLongPtrW,
    GetWindowThreadProcessId, SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE,
    HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
};

use super::LauncherWindow;

pub struct WindowsLauncherWindow {
    /// Captured before the launcher takes the foreground, because Windows has
    /// no non-activating equivalent of an NSPanel and focus must be handed back
    /// explicitly on dismiss.
    previous_foreground: Option<isize>,
}

impl WindowsLauncherWindow {
    pub fn new(window: &WebviewWindow) -> Self {
        if let Some(hwnd) = hwnd(window) {
            unsafe {
                // A tool window is absent from the taskbar and from Alt+Tab,
                // which is the closest Windows gets to macOS accessory policy.
                let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
                SetWindowLongPtrW(
                    hwnd,
                    GWL_EXSTYLE,
                    (style | WS_EX_TOOLWINDOW | WS_EX_TOPMOST) as isize,
                );
                SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
        }
        Self {
            previous_foreground: None,
        }
    }
}

impl LauncherWindow for WindowsLauncherWindow {
    fn show(&mut self, window: &WebviewWindow) {
        unsafe {
            let foreground = GetForegroundWindow();
            if !foreground.is_null() {
                self.previous_foreground = Some(foreground as isize);
            }
        }

        let _ = window.show();

        if let Some(hwnd) = hwnd(window) {
            unsafe {
                SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
                force_foreground(hwnd);
            }
        }
    }

    fn hide(&mut self, window: &WebviewWindow) {
        let _ = window.hide();
    }

    fn position_on_active_display(&self, window: &WebviewWindow) {
        super::position_centred(window);
    }

    fn restore_previous_focus(&mut self) {
        let Some(previous) = self.previous_foreground.take() else {
            return;
        };
        unsafe {
            let hwnd = previous as *mut c_void as HWND;
            AllowSetForegroundWindow(u32::MAX);
            force_foreground(hwnd);
        }
    }
}

/// Windows refuses SetForegroundWindow from a process that does not own the
/// current foreground window. Briefly sharing an input queue with that window's
/// thread lifts the restriction, which is the standard way around foreground
/// lock.
unsafe fn force_foreground(target: HWND) {
    unsafe {
        let foreground = GetForegroundWindow();
        if foreground.is_null() {
            SetForegroundWindow(target);
            return;
        }

        let current = GetCurrentThreadId();
        let owner = GetWindowThreadProcessId(foreground, std::ptr::null_mut());

        if owner == current {
            SetForegroundWindow(target);
            SetFocus(target);
            return;
        }

        AttachThreadInput(current, owner, TRUE);
        SetForegroundWindow(target);
        SetFocus(target);
        AttachThreadInput(current, owner, 0);
    }
}

fn hwnd(window: &WebviewWindow) -> Option<HWND> {
    let handle = window.hwnd().ok()?;
    Some(handle.0 as *mut c_void as HWND)
}
