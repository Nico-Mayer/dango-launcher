mod apps;
mod clipboard;
mod eventtap;
mod hyperkey;
mod icons;
mod keymap;
mod keymonitor;
mod permissions;
mod system;
mod text;
mod window;

pub use apps::MacAppIndexer;
pub use clipboard::MacAttribution;
pub use hyperkey::MacHyperkey;
pub use icons::icon_for;
pub use keymonitor::MacKeyMonitor;
pub use permissions::MacPermissions;
pub use system::{relaunch, MacSystemControl};
pub use text::{MacHandoff, MacKeys, MacSelection};
pub use window::MacWindowManager;

use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, Bool, ClassBuilder, Sel};
use objc2::sel;
use objc2_app_kit::{
    NSEvent, NSScreen, NSView, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_foundation::{MainThreadMarker, NSPoint};
use raw_window_handle::RawWindowHandle;

use super::{LauncherWindow, RunningApp, SystemControl, SystemError};

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

pub struct MacLauncherWindow;

impl MacLauncherWindow {
    pub fn new(window: RawWindowHandle) -> Self {
        if let Some(ns_window) = ns_window(window) {
            reclass_as_panel(&ns_window);

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
        }
        Self
    }
}

impl LauncherWindow for MacLauncherWindow {
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

    /// Centred on the display under the cursor, in AppKit's points, whose
    /// origin is the bottom-left of the primary display.
    fn position_on_active_display(&self, window: RawWindowHandle) {
        let (Some(ns_window), Some(mtm)) = (ns_window(window), MainThreadMarker::new()) else {
            return;
        };
        let cursor = NSEvent::mouseLocation();
        let screens = NSScreen::screens(mtm);
        let screen = (0..screens.count())
            .map(|index| screens.objectAtIndex(index))
            .find(|screen| {
                let frame = screen.frame();
                cursor.x >= frame.origin.x
                    && cursor.x < frame.origin.x + frame.size.width
                    && cursor.y >= frame.origin.y
                    && cursor.y < frame.origin.y + frame.size.height
            })
            .or_else(|| NSScreen::mainScreen(mtm));
        let Some(screen) = screen else {
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
}

extern "C" fn can_become_key_window(_this: NonNull<AnyObject>, _cmd: Sel) -> Bool {
    Bool::YES
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
        }
        Some(builder.register())
    })
}

fn reclass_as_panel(ns_window: &NSWindow) {
    let Some(class) = panel_class() else {
        return;
    };
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
