use tauri::WebviewWindow;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
compile_error!("Dango targets macOS and Windows only. Linux is out of scope.");

pub trait LauncherWindow: Send {
    fn show(&mut self, window: &WebviewWindow);
    fn hide(&mut self, window: &WebviewWindow);
    fn position_on_active_display(&self, window: &WebviewWindow);
    fn restore_previous_focus(&mut self);
}

pub fn launcher_window(window: &WebviewWindow) -> Box<dyn LauncherWindow> {
    #[cfg(target_os = "macos")]
    return Box::new(macos::MacLauncherWindow::new(window));
    #[cfg(target_os = "windows")]
    return Box::new(windows::WindowsLauncherWindow::new(window));
}

/// One running application, as the quit command needs to see it. `id` is
/// whatever the platform addresses it by, opaque above this line.
#[derive(Clone, Debug, PartialEq)]
pub struct RunningApp {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum SystemError {
    #[error("{0}")]
    Failed(String),
    #[error("that application is no longer running")]
    Gone,
    #[error("{0} is not implemented on this platform yet")]
    Unsupported(&'static str),
}

/// The operating-system actions the `system` extension offers. Everything
/// platform-specific about them sits behind this one trait.
pub trait SystemControl: Send + Sync {
    fn lock(&self) -> Result<(), SystemError>;
    fn sleep(&self) -> Result<(), SystemError>;
    /// How many items are in the trash. Zero means emptying it is a no-op, and
    /// the command says so instead of asking for confirmation.
    fn trash_count(&self) -> Result<usize, SystemError>;
    fn empty_trash(&self) -> Result<(), SystemError>;
    fn running_apps(&self) -> Vec<RunningApp>;
    /// Asks an application to quit the ordinary way, so it can prompt about
    /// unsaved work. A refusal is not an error.
    fn quit(&self, app_id: &str) -> Result<(), SystemError>;
}

pub fn system_control() -> std::sync::Arc<dyn SystemControl> {
    #[cfg(target_os = "windows")]
    return std::sync::Arc::new(windows::WindowsSystemControl);
    #[cfg(target_os = "macos")]
    return std::sync::Arc::new(macos::MacSystemControl);
}

/// Renders the icon the system shows for something, at the requested size.
/// `locator` is a filesystem path on macOS and a shell parsing name on Windows,
/// which is what lets one helper serve both the application index and the list
/// of running applications.
pub fn icon_for(locator: &str, size: u32) -> Option<crate::extensions::applications::IconRgba> {
    #[cfg(target_os = "windows")]
    return windows::icon_for(locator, size);
    #[cfg(target_os = "macos")]
    return macos::icon_for(locator, size);
}

/// The application indexer for this platform.
pub fn app_indexer() -> std::sync::Arc<dyn crate::extensions::applications::AppIndexer> {
    #[cfg(target_os = "windows")]
    return std::sync::Arc::new(windows::WindowsAppIndexer);
    #[cfg(target_os = "macos")]
    return std::sync::Arc::new(macos::MacAppIndexer);
}

/// The cursor is the most reliable signal for "the display the user is looking
/// at". `current_monitor` cannot answer while the window is parked offscreen for
/// warmup, and returning nothing there would leave the launcher unpositioned and
/// invisible.
fn active_monitor(window: &WebviewWindow) -> Option<tauri::Monitor> {
    if let Ok(cursor) = window.cursor_position() {
        if let Ok(Some(monitor)) = window.monitor_from_point(cursor.x, cursor.y) {
            return Some(monitor);
        }
    }
    if let Ok(Some(monitor)) = window.current_monitor() {
        return Some(monitor);
    }
    window.primary_monitor().ok().flatten()
}

/// Horizontal centre, above vertical centre, in the display's work area.
pub fn launcher_origin(
    area_x: f64,
    area_y: f64,
    area_width: f64,
    area_height: f64,
    window_width: f64,
    window_height: f64,
) -> (f64, f64) {
    let x = area_x + (area_width - window_width) / 2.0;
    let y = area_y + (area_height - window_height) / 2.0 - area_height * 0.12;
    (x, y.max(area_y))
}

#[cfg(test)]
mod tests {
    use super::launcher_origin;

    #[test]
    fn centres_horizontally() {
        let (x, _) = launcher_origin(0.0, 0.0, 1920.0, 1080.0, 720.0, 420.0);
        assert_eq!(x, 600.0);
    }

    #[test]
    fn sits_above_vertical_centre() {
        let (_, y) = launcher_origin(0.0, 0.0, 1920.0, 1080.0, 720.0, 420.0);
        assert!(y < (1080.0 - 420.0) / 2.0);
    }

    #[test]
    fn offsets_by_secondary_display_origin() {
        let (x, y) = launcher_origin(1920.0, -200.0, 1280.0, 800.0, 720.0, 420.0);
        assert_eq!(x, 1920.0 + (1280.0 - 720.0) / 2.0);
        assert!(y >= -200.0);
    }

    #[test]
    fn never_escapes_above_the_work_area() {
        let (_, y) = launcher_origin(0.0, 100.0, 800.0, 200.0, 720.0, 420.0);
        assert!(y >= 100.0);
    }
}
