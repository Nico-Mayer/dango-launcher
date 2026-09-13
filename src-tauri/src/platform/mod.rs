use tauri::WebviewWindow;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

/// What the walkthrough harness needs to point the exchange at a window of its
/// own and to bring that window forward the same way an insertion does.
#[cfg(target_os = "windows")]
pub use windows::{own_integrity_level, remember_previous_foreground, WindowsHandoff};

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

/// Which physical key acts as the hyperkey. Only CapsLock for now; the allowlist
/// can grow without a config break.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HyperkeyTrigger {
    CapsLock,
}

impl HyperkeyTrigger {
    /// Maps a config `key` name to a trigger, or `None` for a name not on the
    /// allowlist.
    pub fn from_key(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "capslock" | "caps" => Some(Self::CapsLock),
            _ => None,
        }
    }
}

/// The modifiers the hyperkey holds down while the trigger key is held.
#[derive(Clone, Copy, Debug)]
pub struct HyperModifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}

/// A hyperkey to install: which key, and what it emits while held.
#[derive(Clone, Copy, Debug)]
pub struct HyperkeySpec {
    pub trigger: HyperkeyTrigger,
    pub emit: HyperModifiers,
}

/// A running hyperkey remap. Dropping the handle stops the remap and releases
/// any modifiers it is holding, so a reload or shutdown never leaves keys stuck
/// down.
pub trait Hyperkey: Send {}

/// Starts the hyperkey for this platform, or `None` when it cannot run: the OS
/// refused the interceptor, or this platform has no implementation yet.
pub fn start_hyperkey(spec: HyperkeySpec) -> Option<Box<dyn Hyperkey>> {
    #[cfg(target_os = "windows")]
    {
        windows::WindowsHyperkey::start(spec).map(|h| Box::new(h) as Box<dyn Hyperkey>)
    }
    #[cfg(target_os = "macos")]
    {
        // The macOS event tap is a later task; a configured hyperkey reports
        // unavailable rather than silently doing nothing.
        let _ = spec;
        None
    }
}

/// The selection and paste path for this platform, or `None` when key injection
/// is unavailable and nothing here can work.
///
/// `own_writes` is how the clipboard history is told to ignore what this
/// borrows. Pass `crate::text::Unwatched` when the clipboard extension is off.
pub fn text_exchange(
    clipboard: std::sync::Arc<dyn crate::extensions::clipboard::ClipboardSource>,
    own_writes: std::sync::Arc<dyn crate::text::OwnWrites>,
    launcher: std::sync::Arc<dyn crate::text::Launcher>,
    main: std::sync::Arc<dyn crate::text::MainThread>,
) -> Option<crate::text::TextExchange> {
    #[cfg(target_os = "macos")]
    {
        let keys = std::sync::Arc::new(macos::MacKeys::new(main)?);
        Some(crate::text::TextExchange::new(
            clipboard,
            keys,
            std::sync::Arc::new(macos::MacHandoff),
            Some(std::sync::Arc::new(macos::MacSelection)),
            own_writes,
            launcher,
        ))
    }
    #[cfg(target_os = "windows")]
    {
        // Windows has no main-thread rule for key synthesis: `SendInput` and
        // the layout lookup are callable from any thread.
        let _ = main;
        let keys = std::sync::Arc::new(windows::WindowsKeys::new()?);
        Some(crate::text::TextExchange::new(
            clipboard,
            keys,
            std::sync::Arc::new(windows::WindowsHandoff),
            // No accessibility route worth having here: the UI Automation text
            // pattern only answers in applications that implement it, and the
            // clipboard round trip has to be right anyway.
            None,
            own_writes,
            launcher,
        ))
    }
}

/// Which application did the copying. The clipboard itself is a crate; this is
/// the one question it cannot answer.
///
/// On macOS this also starts recording application activations, which must
/// happen on the main thread, so it is called during setup rather than from the
/// extension's background service.
pub fn attribution() -> std::sync::Arc<dyn crate::extensions::clipboard::Attribution> {
    #[cfg(target_os = "windows")]
    return std::sync::Arc::new(windows::WindowsAttribution);
    #[cfg(target_os = "macos")]
    {
        let attribution = macos::MacAttribution::new();
        attribution.watch_activations();
        std::sync::Arc::new(attribution)
    }
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

/// A window rectangle in the platform's own coordinates: physical pixels across
/// one virtual desktop on Windows, logical points per screen on macOS. The
/// geometry above this line never converts between the two, because the trait
/// reports and accepts whatever the platform speaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// A snapshot of the window a command is about to reshape: its visible frame and
/// the work area of the display it is currently on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placement {
    pub frame: Rect,
    pub work_area: Rect,
}

#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
pub enum WindowError {
    #[error("there is no window to move")]
    NoTarget,
    #[error("{0}")]
    Unreachable(String),
    #[error("{0}")]
    Failed(String),
}

/// Reshaping the window that was focused before the launcher appeared. Narrow on
/// purpose: read the target's frame and its display, list the displays, and set
/// the frame. Every region computation lives above this in shared, tested code.
pub trait WindowManager: Send + Sync {
    /// The target window's visible frame and the work area of its display, or
    /// `NoTarget` if there is no window to act on.
    fn target(&self) -> Result<Placement, WindowError>;
    /// The work areas of all displays, in a stable order, for move-to-next.
    fn displays(&self) -> Vec<Rect>;
    /// Move and resize the target window to `frame`.
    fn place(&self, frame: Rect) -> Result<(), WindowError>;
}

/// `main` is how macOS reaches `NSScreen`, which only answers on the main
/// thread. Windows has no such rule and ignores it.
pub fn window_manager(
    main: std::sync::Arc<dyn crate::text::MainThread>,
) -> std::sync::Arc<dyn WindowManager> {
    #[cfg(target_os = "windows")]
    {
        let _ = main;
        std::sync::Arc::new(windows::WindowsWindowManager)
    }
    #[cfg(target_os = "macos")]
    {
        std::sync::Arc::new(macos::MacWindowManager::new(main))
    }
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
    use super::{Hyperkey, HyperkeyTrigger};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    struct FakeHyperkey {
        stopped: Arc<AtomicBool>,
    }
    impl Hyperkey for FakeHyperkey {}
    impl Drop for FakeHyperkey {
        fn drop(&mut self) {
            self.stopped.store(true, Ordering::SeqCst);
        }
    }

    #[test]
    fn a_hyperkey_handle_stops_on_drop() {
        let stopped = Arc::new(AtomicBool::new(false));
        let handle: Box<dyn Hyperkey> = Box::new(FakeHyperkey {
            stopped: stopped.clone(),
        });
        assert!(!stopped.load(Ordering::SeqCst));
        drop(handle);
        assert!(
            stopped.load(Ordering::SeqCst),
            "dropping the handle stops it"
        );
    }

    #[test]
    fn trigger_from_key_uses_an_allowlist() {
        assert_eq!(
            HyperkeyTrigger::from_key("capslock"),
            Some(HyperkeyTrigger::CapsLock)
        );
        assert_eq!(
            HyperkeyTrigger::from_key("CapsLock"),
            Some(HyperkeyTrigger::CapsLock)
        );
        assert!(HyperkeyTrigger::from_key("f13").is_none());
    }

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
