mod apps;
mod clipboard;
mod icons;
mod system;
mod text;

pub use apps::MacAppIndexer;
pub use clipboard::MacAttribution;
pub use icons::icon_for;
pub use system::MacSystemControl;
pub use text::{MacHandoff, MacKeys, MacSelection};

use std::ptr::NonNull;
use std::sync::OnceLock;

use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, Bool, ClassBuilder, Sel};
use objc2::sel;
use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask};
use tauri::WebviewWindow;

use super::{LauncherWindow, RunningApp, SystemControl, SystemError};

/// Above NSMainMenuWindowLevel. A fullscreen application's window outranks the
/// floating level that `alwaysOnTop` gives us, which leaves the launcher behind
/// it.
const NS_POPUP_MENU_WINDOW_LEVEL: isize = 101;

pub struct MacLauncherWindow;

impl MacLauncherWindow {
    pub fn new(window: &WebviewWindow) -> Self {
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
    fn show(&mut self, window: &WebviewWindow) {
        let _ = window.show();
        // Ordering is re-asserted after the show, because Tauri reapplies its
        // own always-on-top level on the way out and would clobber ours.
        if let Some(ns_window) = ns_window(window) {
            ns_window.setLevel(NS_POPUP_MENU_WINDOW_LEVEL);
            ns_window.orderFrontRegardless();
            ns_window.makeKeyWindow();
        }
    }

    fn hide(&mut self, window: &WebviewWindow) {
        let _ = window.hide();
    }

    /// macOS positions in logical points, and each display's physical
    /// coordinates are its own points times its own scale, so centring has to
    /// stay in logical space.
    fn position_on_active_display(&self, window: &WebviewWindow) {
        let Some(monitor) = super::active_monitor(window) else {
            return;
        };
        let scale = monitor.scale_factor();
        let area = monitor.size().to_logical::<f64>(scale);
        let origin = monitor.position().to_logical::<f64>(scale);
        let Ok(size) = window.outer_size() else {
            return;
        };
        let size = size.to_logical::<f64>(scale);

        let (x, y) = super::launcher_origin(
            origin.x,
            origin.y,
            area.width,
            area.height,
            size.width,
            size.height,
        );
        let _ = window.set_position(tauri::LogicalPosition::new(x, y));
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
/// The subclass exists because reclassing drops tao's `canBecomeKeyWindow`
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

fn ns_window(window: &WebviewWindow) -> Option<Retained<NSWindow>> {
    let ptr = window.ns_window().ok()? as *mut AnyObject;
    if ptr.is_null() {
        return None;
    }
    unsafe { Retained::retain(ptr.cast::<NSWindow>()) }
}
