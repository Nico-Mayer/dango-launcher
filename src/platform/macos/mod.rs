mod agent;
mod apps;
mod brightness;
mod browser;
mod clipboard;
mod eventtap;
mod files;
mod hyperkey;
mod icons;
mod keymap;
mod keymonitor;
mod permissions;
mod reopen;
mod system;
mod text;
mod window;

pub use agent::{agent_known_dirs, contained_command, login_shell_env};
pub use apps::MacAppIndexer;
pub use brightness::MacBrightness;
pub use browser::{browser_app, browser_target};
pub use clipboard::MacAttribution;
pub use files::{reveal, screenshot_folder};
pub use hyperkey::MacHyperkey;
pub use icons::icon_for;
pub use keymonitor::MacKeyMonitor;
pub use permissions::MacPermissions;
pub use reopen::watch_reopen;
pub use system::{relaunch, MacSystemControl};
pub use text::{MacHandoff, MacKeys, MacSelection};
pub use window::MacWindowManager;

use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::OnceLock;

use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, Bool, ClassBuilder, Sel};
use objc2::{msg_send, sel};
use objc2_app_kit::{
    NSAnimatablePropertyContainer, NSAnimationContext, NSEvent, NSScreen, NSView, NSWindow,
    NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_foundation::{MainThreadMarker, NSPoint};
use raw_window_handle::RawWindowHandle;

use super::{LauncherWindow, NoticeWindow, RunningApp, SystemControl, SystemError};
use crate::config::LauncherDisplay;

/// Whether the Accessibility permission has been granted. macOS gates every
/// route into another application behind it: key synthesis, reading the
/// selection, and moving a window.
pub(super) fn accessibility_trusted() -> bool {
    unsafe { objc2_application_services::AXIsProcessTrusted() }
}

/// Asks the system to show its Accessibility prompt, which is the only way to
/// offer the user a route to the settings pane. Called from an action the user
/// chose, never on its own: a dialog from a process they cannot see is worse
/// than a command that explains itself.
pub(super) fn prompt_for_accessibility() {
    use objc2_core_foundation::{CFBoolean, CFDictionary, CFRetained, CFString};

    let prompt = CFString::from_static_str("AXTrustedCheckOptionPrompt");
    unsafe {
        let Some(yes) = objc2_core_foundation::kCFBooleanTrue else {
            return;
        };
        let options: CFRetained<CFDictionary<CFString, CFBoolean>> =
            CFDictionary::from_slices(&[prompt.as_ref()], &[yes]);
        objc2_application_services::AXIsProcessTrustedWithOptions(Some(options.as_opaque()));
    }
}

/// Marks an event as Dango's own, in the event source's user data. The same
/// value the Windows side stamps in `dwExtraInfo`, so one number means one thing
/// on both platforms.
pub(super) const DANGO_INJECTED: i64 = 0x44_41_4E_47;

/// How many injections are in flight. A depth rather than a flag, so a nested
/// injection cannot unmute the monitor early.
static MUTED: AtomicUsize = AtomicUsize::new(0);

/// Whether the key monitor should ignore what it is seeing, because Dango is
/// currently typing it.
pub(super) fn muted() -> bool {
    MUTED.load(Ordering::SeqCst) > 0
}

/// Mutes the key monitor while it is held. macOS cannot mark the keystrokes
/// enigo injects, so expansion mutes the monitor around them instead of
/// filtering them; see the keyword-expansion design.
pub struct Mute;

impl Mute {
    pub fn new() -> Self {
        MUTED.fetch_add(1, Ordering::SeqCst);
        Self
    }
}

impl Drop for Mute {
    fn drop(&mut self) {
        MUTED.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Whether the launcher panel is the key window. The panel never makes Dango
/// frontmost and the key event does not name Dango as its target, so this is
/// how the key monitor tells that keys are aimed at the launcher.
static PANEL_IS_KEY: AtomicBool = AtomicBool::new(false);

pub(super) fn panel_is_key() -> bool {
    PANEL_IS_KEY.load(Ordering::SeqCst)
}

/// The frontmost application's name, resolved the same way the clipboard names
/// the copying application, so the one exclusion list matches both.
pub(super) fn foreground_app() -> Option<String> {
    clipboard::frontmost_name()
}

/// Whether the system has secure event input active, which is what a password
/// field turns on. Authoritative here, unlike the Windows guess: while it is on
/// the tap receives nothing useful anyway, and asking makes the skip deliberate
/// rather than an accident of what was delivered.
pub(super) fn focused_field_is_secure() -> bool {
    #[link(name = "Carbon", kind = "framework")]
    extern "C" {
        fn IsSecureEventInputEnabled() -> u8;
    }
    unsafe { IsSecureEventInputEnabled() != 0 }
}

/// Above NSMainMenuWindowLevel. A fullscreen application's window outranks the
/// floating level that `alwaysOnTop` gives us, which leaves the launcher behind
/// it.
const NS_POPUP_MENU_WINDOW_LEVEL: isize = 101;

/// Above the launcher, so a notice shown as the launcher hides is never under it.
const NOTICE_WINDOW_LEVEL: isize = NS_POPUP_MENU_WINDOW_LEVEL + 1;

/// Matches `Theme.reveal-duration`, in seconds.
const REVEAL_SECONDS: f64 = 0.12;

pub struct MacLauncherWindow;

impl LauncherWindow for MacLauncherWindow {
    fn attach(&mut self, window: RawWindowHandle) {
        if let Some(ns_window) = ns_window(window) {
            if let Some(class) = panel_class() {
                reclass_as_panel(&ns_window, class);
            }

            // Joining all spaces plus fullscreen-auxiliary lets the launcher
            // draw into another app's fullscreen space instead of making the
            // system switch away from it.
            ns_window.setCollectionBehavior(
                NSWindowCollectionBehavior::CanJoinAllSpaces
                    | NSWindowCollectionBehavior::FullScreenAuxiliary
                    | NSWindowCollectionBehavior::IgnoresCycle,
            );
            ns_window.setLevel(NS_POPUP_MENU_WINDOW_LEVEL);
            ns_window.setHidesOnDeactivate(false);
            // winit leaves AppKit's window shadow on. It traces the window's
            // alpha, Slint shadow included, and is not recomputed when the
            // card changes height, since the window never resizes here.
            ns_window.setHasShadow(false);
        }
    }

    fn will_show(&mut self) {}

    fn did_show(&mut self, window: RawWindowHandle) {
        // Ordering is re-asserted after the show, because the toolkit reapplies
        // its own always-on-top level on the way out and would clobber ours.
        if let Some(ns_window) = ns_window(window) {
            ns_window.setLevel(NS_POPUP_MENU_WINDOW_LEVEL);
            ns_window.orderFrontRegardless();
            ns_window.makeKeyWindow();
        }
    }

    /// In AppKit's points, whose origin is the bottom-left of the primary
    /// display.
    fn position_on_active_display(&self, window: RawWindowHandle, display: LauncherDisplay) {
        let (Some(ns_window), Some(screen)) = (ns_window(window), target_screen(display)) else {
            return;
        };
        let area = screen.frame();
        let size = ns_window.frame().size;
        // Measured from the display's top edge, then flipped.
        let (x, from_top) = super::launcher_origin(
            area.origin.x,
            0.0,
            area.size.width,
            area.size.height,
            size.width,
            size.height,
        );
        let y = area.origin.y + area.size.height - from_top - size.height;
        ns_window.setFrameOrigin(NSPoint::new(x, y));
    }

    fn restore_previous_focus(&mut self) {
        // A non-activating panel never takes application activation, so there
        // is nothing to hand back.
    }

    /// Through the animator, so it replaces a fade still running from the
    /// last show; a plain set can be overtaken by it.
    fn conceal(&mut self, window: RawWindowHandle) {
        if let Some(ns_window) = ns_window(window) {
            animate_alpha(&ns_window, 0.0, 0.0);
        }
    }

    fn reveal(&mut self, window: RawWindowHandle) {
        if let Some(ns_window) = ns_window(window) {
            animate_alpha(&ns_window, 1.0, REVEAL_SECONDS);
        }
    }
}

fn animate_alpha(ns_window: &NSWindow, alpha: f64, seconds: f64) {
    let change = block2::RcBlock::new(|context: NonNull<NSAnimationContext>| {
        unsafe { context.as_ref() }.setDuration(seconds);
        ns_window.animator().setAlphaValue(alpha);
    });
    NSAnimationContext::runAnimationGroup(&change);
}

pub struct MacNoticeWindow;

impl NoticeWindow for MacNoticeWindow {
    /// A plain NSPanel, not the launcher's class: that one can become key and
    /// records it in `PANEL_IS_KEY`. A borderless NSPanel never becomes key.
    fn attach(&mut self, window: RawWindowHandle) {
        let (Some(ns_window), Some(class)) = (ns_window(window), AnyClass::get(c"NSPanel")) else {
            return;
        };
        reclass_as_panel(&ns_window, class);
        ns_window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary
                | NSWindowCollectionBehavior::IgnoresCycle
                | NSWindowCollectionBehavior::Stationary,
        );
        ns_window.setLevel(NOTICE_WINDOW_LEVEL);
        ns_window.setHidesOnDeactivate(false);
    }

    /// In AppKit's points, whose origin is the bottom-left of the primary
    /// display, so the margin is measured up from the visible frame's bottom.
    fn place(&self, window: RawWindowHandle, display: LauncherDisplay) {
        let (Some(ns_window), Some(screen)) = (ns_window(window), target_screen(display)) else {
            return;
        };
        let area = screen.visibleFrame();
        let size = ns_window.frame().size;
        let x = area.origin.x + (area.size.width - size.width) / 2.0;
        let y = area.origin.y + super::NOTICE_MARGIN;
        ns_window.setFrameOrigin(NSPoint::new(x, y));
    }

    /// The toolkit reapplies its own always-on-top level on show.
    fn did_show(&self, window: RawWindowHandle) {
        if let Some(ns_window) = ns_window(window) {
            ns_window.setLevel(NOTICE_WINDOW_LEVEL);
        }
    }
}

fn target_screen(display: LauncherDisplay) -> Option<Retained<NSScreen>> {
    let mtm = MainThreadMarker::new()?;
    match display {
        LauncherDisplay::Pointer => screen_under_cursor(mtm),
        LauncherDisplay::Primary => primary_screen(mtm),
    }
}

/// `mainScreen` is the screen with the key window, not the one set as main in
/// System Settings > Displays; that one is always first.
fn primary_screen(mtm: MainThreadMarker) -> Option<Retained<NSScreen>> {
    NSScreen::screens(mtm).firstObject()
}

fn screen_under_cursor(mtm: MainThreadMarker) -> Option<Retained<NSScreen>> {
    let cursor = NSEvent::mouseLocation();
    let screens = NSScreen::screens(mtm);
    (0..screens.count())
        .map(|index| screens.objectAtIndex(index))
        .find(|screen| {
            let frame = screen.frame();
            cursor.x >= frame.origin.x
                && cursor.x < frame.origin.x + frame.size.width
                && cursor.y >= frame.origin.y
                && cursor.y < frame.origin.y + frame.size.height
        })
        .or_else(|| primary_screen(mtm))
}

extern "C" fn can_become_key_window(_this: NonNull<AnyObject>, _cmd: Sel) -> Bool {
    Bool::YES
}

extern "C" fn become_key_window(this: NonNull<AnyObject>, _cmd: Sel) {
    PANEL_IS_KEY.store(true, Ordering::SeqCst);
    if let Some(superclass) = AnyClass::get(c"NSPanel") {
        let _: () = unsafe { msg_send![super(this.as_ref(), superclass), becomeKeyWindow] };
    }
}

extern "C" fn resign_key_window(this: NonNull<AnyObject>, _cmd: Sel) {
    PANEL_IS_KEY.store(false, Ordering::SeqCst);
    if let Some(superclass) = AnyClass::get(c"NSPanel") {
        let _: () = unsafe { msg_send![super(this.as_ref(), superclass), resignKeyWindow] };
    }
}

/// NSPanel accepts the non-activating style mask that a plain NSWindow rejects.
/// The subclass exists because reclassing drops the toolkit's `canBecomeKeyWindow`
/// override, and a borderless window answers NO by default, which would leave
/// the launcher unable to receive typing.
fn panel_class() -> Option<&'static AnyClass> {
    static CLASS: OnceLock<Option<&'static AnyClass>> = OnceLock::new();
    *CLASS.get_or_init(|| {
        if let Some(existing) = AnyClass::get(c"DangoPanel") {
            return Some(existing);
        }
        let superclass = AnyClass::get(c"NSPanel")?;
        let mut builder = ClassBuilder::new(c"DangoPanel", superclass)?;
        unsafe {
            builder.add_method(
                sel!(canBecomeKeyWindow),
                can_become_key_window as extern "C" fn(NonNull<AnyObject>, Sel) -> Bool,
            );
            builder.add_method(
                sel!(becomeKeyWindow),
                become_key_window as extern "C" fn(NonNull<AnyObject>, Sel),
            );
            builder.add_method(
                sel!(resignKeyWindow),
                resign_key_window as extern "C" fn(NonNull<AnyObject>, Sel),
            );
        }
        Some(builder.register())
    })
}

fn reclass_as_panel(ns_window: &NSWindow, class: &AnyClass) {
    let obj: *const NSWindow = ns_window;
    unsafe {
        objc2::ffi::object_setClass(obj as *mut AnyObject as *mut _, class as *const _ as *mut _);
        let mask = ns_window.styleMask() | NSWindowStyleMask::NonactivatingPanel;
        ns_window.setStyleMask(mask);
    }
}

fn ns_window(window: RawWindowHandle) -> Option<Retained<NSWindow>> {
    let RawWindowHandle::AppKit(handle) = window else {
        return None;
    };
    let view = unsafe { handle.ns_view.cast::<NSView>().as_ref() };
    view.window()
}

pub fn instance_lock(data_dir: &std::path::Path) -> String {
    // A failure here surfaces when the lock file cannot be created.
    let _ = std::fs::create_dir_all(data_dir);
    data_dir
        .join("instance.lock")
        .to_string_lossy()
        .into_owned()
}
