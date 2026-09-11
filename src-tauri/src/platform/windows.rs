use tauri::WebviewWindow;
use windows_sys::Win32::Foundation::{FALSE, HWND, TRUE};
use windows_sys::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, GetForegroundWindow, GetWindowLongPtrW, GetWindowThreadProcessId,
    SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, HWND_TOPMOST,
    SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST,
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
            unsafe { apply_tool_window_style(hwnd) };
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
                apply_tool_window_style(hwnd);
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
            let hwnd = previous as HWND;
            AllowSetForegroundWindow(u32::MAX);
            force_foreground(hwnd);
        }
    }
}

/// tao rewrites the extended style from its own flags whenever visibility
/// changes, which restores WS_EX_APPWINDOW and drops the tool window bit. So
/// this runs after every show, not once at startup.
unsafe fn apply_tool_window_style(hwnd: HWND) {
    let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
    let style = (style | WS_EX_TOOLWINDOW | WS_EX_TOPMOST) & !WS_EX_APPWINDOW;
    SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style as isize);
    SetWindowPos(
        hwnd,
        HWND_TOPMOST,
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
    );
}

/// Windows refuses SetForegroundWindow from a process that does not own the
/// current foreground window. Briefly sharing an input queue with that window's
/// thread lifts the restriction, which is the standard way around foreground
/// lock.
unsafe fn force_foreground(target: HWND) {
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
    AttachThreadInput(current, owner, FALSE);
}

fn hwnd(window: &WebviewWindow) -> Option<HWND> {
    // Tauri hands back the windows crate HWND, whose inner pointer is exactly
    // the windows-sys HWND, so no conversion is involved.
    Some(window.hwnd().ok()?.0)
}
