use tauri::WebviewWindow;

use super::LauncherWindow;

pub struct WindowsLauncherWindow;

impl WindowsLauncherWindow {
    pub fn new(_window: &WebviewWindow) -> Self {
        Self
    }
}

impl LauncherWindow for WindowsLauncherWindow {
    fn show(&mut self, window: &WebviewWindow) {
        let _ = window.show();
        let _ = window.set_focus();
    }

    fn hide(&mut self, window: &WebviewWindow) {
        let _ = window.hide();
    }

    fn position_on_active_display(&self, window: &WebviewWindow) {
        super::position_centred(window);
    }

    fn restore_previous_focus(&mut self) {}
}
