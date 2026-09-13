mod apps;
mod clipboard;
mod system;
mod text;

pub use apps::{icon_for, WindowsAppIndexer};
pub use clipboard::WindowsAttribution;
pub use system::WindowsSystemControl;
pub use text::{own_integrity_level, remember_previous_foreground, WindowsHandoff, WindowsKeys};

use tauri::WebviewWindow;
use windows_sys::Win32::Foundation::{FALSE, HWND, TRUE};
use windows_sys::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, BringWindowToTop, GetForegroundWindow, GetWindowLongPtrW,
    GetWindowThreadProcessId, SetForegroundWindow, SetWindowLongPtrW, SetWindowPos,
    SystemParametersInfoW, GWL_EXSTYLE, HWND_TOPMOST, SPIF_SENDCHANGE,
    SPI_GETFOREGROUNDLOCKTIMEOUT, SPI_SETFOREGROUNDLOCKTIMEOUT, SWP_FRAMECHANGED, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
};

use super::{LauncherWindow, RunningApp, SystemControl, SystemError};

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
                text::remember_previous_foreground(foreground as isize);
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

    /// Windows uses one physical pixel space across displays, so the window's
    /// size is projected to the target display's DPI before centring. Going
    /// through logical coordinates would scale by the display the window is
    /// leaving, not the one it is moving to.
    fn position_on_active_display(&self, window: &WebviewWindow) {
        let Some(monitor) = super::active_monitor(window) else {
            return;
        };
        let (Ok(size), Ok(scale)) = (window.outer_size(), window.scale_factor()) else {
            return;
        };
        let size = size
            .to_logical::<f64>(scale)
            .to_physical::<f64>(monitor.scale_factor());
        let area = monitor.work_area();

        let (x, y) = super::launcher_origin(
            area.position.x as f64,
            area.position.y as f64,
            area.size.width as f64,
            area.size.height as f64,
            size.width,
            size.height,
        );
        let _ = window.set_position(tauri::PhysicalPosition::new(
            x.round() as i32,
            y.round() as i32,
        ));
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
///
/// Deliberately no SetFocus on the target: activation already hands focus to
/// the WebView2 child, and re-focusing the top-level window afterwards pulls it
/// away for an instant, which fires blur in the page and dismisses the launcher
/// as soon as it shows.
pub(super) unsafe fn force_foreground(target: HWND) {
    // The lock is time based: after the launcher took the foreground, the system
    // refuses to give it away again until a timeout passes, and the request is
    // dropped silently. Zero the timeout for the duration and put it back, so
    // the call below is allowed rather than quietly ignored. This is the piece
    // the attach trick alone did not cover, and the reason a paste sometimes
    // reported that the previous window would not come forward.
    let mut previous_timeout: u32 = 0;
    SystemParametersInfoW(
        SPI_GETFOREGROUNDLOCKTIMEOUT,
        0,
        (&mut previous_timeout as *mut u32).cast(),
        0,
    );
    SystemParametersInfoW(
        SPI_SETFOREGROUNDLOCKTIMEOUT,
        0,
        std::ptr::null_mut(),
        SPIF_SENDCHANGE,
    );

    // Share an input queue with both the window losing the foreground and the
    // one gaining it. Attaching to the target's thread is what lets the focus
    // actually land there; attaching to the current foreground lifts its claim.
    // The foreground can be null in the instant after the launcher hides, so
    // that case still has to relax the lock rather than fall through untreated.
    let current = GetCurrentThreadId();
    let target_thread = GetWindowThreadProcessId(target, std::ptr::null_mut());
    let foreground = GetForegroundWindow();
    let foreground_thread = if foreground.is_null() {
        0
    } else {
        GetWindowThreadProcessId(foreground, std::ptr::null_mut())
    };

    let attach_fg = foreground_thread != 0 && foreground_thread != current;
    let attach_target = target_thread != 0 && target_thread != current;
    if attach_fg {
        AttachThreadInput(current, foreground_thread, TRUE);
    }
    if attach_target {
        AttachThreadInput(current, target_thread, TRUE);
    }

    SetForegroundWindow(target);
    BringWindowToTop(target);

    if attach_target {
        AttachThreadInput(current, target_thread, FALSE);
    }
    if attach_fg {
        AttachThreadInput(current, foreground_thread, FALSE);
    }

    SystemParametersInfoW(
        SPI_SETFOREGROUNDLOCKTIMEOUT,
        0,
        previous_timeout as usize as *mut _,
        SPIF_SENDCHANGE,
    );
}

fn hwnd(window: &WebviewWindow) -> Option<HWND> {
    // Tauri hands back the windows crate HWND, whose inner pointer is exactly
    // the windows-sys HWND, so no conversion is involved.
    Some(window.hwnd().ok()?.0)
}
