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
