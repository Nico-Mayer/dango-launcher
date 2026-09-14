//! The macOS half of window management.
//!
//! Two macOS facts shape it. Another application's window can only be reached
//! through the Accessibility API, so every read and write here is an attribute
//! on the frontmost application's `AXFocusedWindow`, and all of it is gated
//! behind the Accessibility permission. And the two coordinate spaces disagree:
//! Accessibility measures logical points down from the top-left of the primary
//! screen, while `NSScreen` measures up from its bottom-left, so every screen
//! rectangle is flipped on the way in and the geometry above this line sees one
//! space.
//!
//! The target is the frontmost application's focused window rather than a
//! remembered handle, because the launcher is a non-activating panel: it never
//! took activation, so the application the user was in is still the frontmost
//! one while the launcher is on screen.

use std::ptr::NonNull;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

use objc2_app_kit::{NSScreen, NSWorkspace};
use objc2_application_services::{AXError, AXUIElement, AXValue, AXValueType};
use objc2_core_foundation::{CFRetained, CFString, CFType, CGPoint, CGRect, CGSize};
use objc2_foundation::MainThreadMarker;

use crate::platform::{Placement, Rect, WindowError, WindowManager, WINDOW_READ_FAILED};
use crate::text::MainThread;

const FOCUSED_WINDOW: &str = "AXFocusedWindow";
const POSITION: &str = "AXPosition";
const SIZE: &str = "AXSize";

const PERMISSION_MISSING: &str = "Dango needs the Accessibility permission to move windows. Grant it in System Settings, Privacy & Security.";
const NOT_MOVABLE: &str = "That app doesn't let Dango move its windows.";

/// Long enough that a briefly busy main thread still answers, short enough that
/// a wedged one fails rather than hanging the command.
const MAIN_THREAD_TIMEOUT: Duration = Duration::from_secs(2);

/// One display, in Accessibility coordinates.
#[derive(Clone, Copy)]
struct Screen {
    frame: Rect,
    work_area: Rect,
}

/// `NSScreen` is main-thread only, so the manager carries the same hop the rest
/// of the macOS code uses. Nothing else here needs it: Accessibility calls are
/// answerable from any thread.
pub struct MacWindowManager {
    main: Arc<dyn MainThread>,
}

impl MacWindowManager {
    pub fn new(main: Arc<dyn MainThread>) -> Self {
        Self { main }
    }

    fn screens(&self) -> Vec<Screen> {
        let (done, wait) = mpsc::channel();
        self.main.run(Box::new(move || {
            let screens = MainThreadMarker::new()
                .map(read_screens)
                .unwrap_or_default();
            let _ = done.send(screens);
        }));
        wait.recv_timeout(MAIN_THREAD_TIMEOUT).unwrap_or_default()
    }
}

impl WindowManager for MacWindowManager {
    fn target(&self) -> Result<Placement, WindowError> {
        let window = focused_window()?;
        let frame = read_frame(&window)?;
        let work_area = work_area_for(frame, &self.screens()).ok_or_else(|| {
            eprintln!("[dango] no screen contains the window's frame");
            WindowError::Failed(WINDOW_READ_FAILED.into())
        })?;
        Ok(Placement { frame, work_area })
    }

    fn displays(&self) -> Vec<Rect> {
        self.screens().into_iter().map(|s| s.work_area).collect()
    }

    fn place(&self, frame: Rect) -> Result<(), WindowError> {
        let window = focused_window()?;
        let position = CGPoint::new(frame.x as f64, frame.y as f64);
        let size = CGSize::new(frame.width as f64, frame.height as f64);

        write_value(&window, POSITION, AXValueType::CGPoint, position)?;
        write_value(&window, SIZE, AXValueType::CGSize, size)?;
        // A resize can shove a window that no longer fits where it was, so the
        // origin is asserted once more after the size is in.
        write_value(&window, POSITION, AXValueType::CGPoint, position)
    }
}

/// The frontmost application's focused window, or why there is none.
fn focused_window() -> Result<CFRetained<AXUIElement>, WindowError> {
    if !super::accessibility_trusted() {
        super::prompt_for_accessibility();
        return Err(WindowError::Failed(PERMISSION_MISSING.into()));
    }
    let app = NSWorkspace::sharedWorkspace()
        .frontmostApplication()
        .ok_or(WindowError::NoTarget)?;
    let pid = app.processIdentifier();
    // Dango being frontmost means the user activated it deliberately; there is
    // no window behind it to act on, and its own is never the target.
    if pid == std::process::id() as i32 {
        return Err(WindowError::NoTarget);
    }
    let element = unsafe { AXUIElement::new_application(pid) };
    unsafe { copy_attribute(&element, FOCUSED_WINDOW) }
        .map_err(ax_error)?
        .ok_or(WindowError::NoTarget)?
        .downcast::<AXUIElement>()
        .map_err(|_| {
            eprintln!("[dango] the focused window was not an AXUIElement");
            WindowError::Failed(WINDOW_READ_FAILED.into())
        })
}

fn read_frame(window: &AXUIElement) -> Result<Rect, WindowError> {
    let position: CGPoint = read_value(window, POSITION, AXValueType::CGPoint)?;
    let size: CGSize = read_value(window, SIZE, AXValueType::CGSize)?;
    Ok(Rect {
        x: position.x.round() as i32,
        y: position.y.round() as i32,
        width: size.width.round() as i32,
        height: size.height.round() as i32,
    })
}

fn read_value<T: Default>(
    window: &AXUIElement,
    attribute: &str,
    kind: AXValueType,
) -> Result<T, WindowError> {
    let value = unsafe { copy_attribute(window, attribute) }
        .map_err(ax_error)?
        .ok_or_else(|| {
            eprintln!("[dango] that window has no {attribute}");
            WindowError::Unreachable(NOT_MOVABLE.into())
        })?
        .downcast::<AXValue>()
        .map_err(|_| {
            eprintln!("[dango] {attribute} was not an AXValue");
            WindowError::Failed(WINDOW_READ_FAILED.into())
        })?;

    let mut out = T::default();
    let decoded = unsafe { value.value(kind, NonNull::from(&mut out).cast()) };
    decoded.then_some(out).ok_or_else(|| {
        eprintln!("[dango] {attribute} could not be read");
        WindowError::Failed(WINDOW_READ_FAILED.into())
    })
}

fn write_value<T>(
    window: &AXUIElement,
    attribute: &str,
    kind: AXValueType,
    mut value: T,
) -> Result<(), WindowError> {
    let boxed =
        unsafe { AXValue::new(kind, NonNull::from(&mut value).cast()) }.ok_or_else(|| {
            eprintln!("[dango] {attribute} could not be encoded");
            WindowError::Failed(WINDOW_READ_FAILED.into())
        })?;
    let name = CFString::from_str(attribute);
    let value: &CFType = &boxed;
    match unsafe { window.set_attribute_value(&name, value) } {
        AXError::Success => Ok(()),
        other => Err(ax_error(other)),
    }
}

unsafe fn copy_attribute(
    element: &AXUIElement,
    attribute: &str,
) -> Result<Option<CFRetained<CFType>>, AXError> {
    let name = CFString::from_str(attribute);
    let mut value: *const CFType = std::ptr::null();
    let status = unsafe { element.copy_attribute_value(&name, NonNull::from(&mut value)) };
    match status {
        AXError::Success => {
            Ok(NonNull::new(value.cast_mut()).map(|value| unsafe { CFRetained::from_raw(value) }))
        }
        AXError::NoValue | AXError::AttributeUnsupported => Ok(None),
        other => Err(other),
    }
}

/// Accessibility's failures, in the vocabulary the commands report. The
/// permission can be revoked between the check and the call, so `APIDisabled`
/// says the same thing the check does.
fn ax_error(error: AXError) -> WindowError {
    match error {
        AXError::APIDisabled => WindowError::Failed(PERMISSION_MISSING.into()),
        AXError::NotImplemented => WindowError::Unreachable(NOT_MOVABLE.into()),
        AXError::InvalidUIElement => WindowError::NoTarget,
        other => {
            eprintln!("[dango] accessibility error {}", other.0);
            WindowError::Failed("Couldn't reach that window. Try again.".into())
        }
    }
}

fn read_screens(mtm: MainThreadMarker) -> Vec<Screen> {
    let screens = NSScreen::screens(mtm);
    // Accessibility hangs its origin on the top-left of the primary screen,
    // which is the one `screens` reports first, so every rectangle flips around
    // that screen's height.
    let Some(primary) = (screens.count() > 0).then(|| screens.objectAtIndex(0)) else {
        return Vec::new();
    };
    let flip = primary.frame().size.height;

    let mut out: Vec<Screen> = (0..screens.count())
        .map(|index| {
            let screen = screens.objectAtIndex(index);
            Screen {
                frame: flipped(screen.frame(), flip),
                work_area: flipped(screen.visibleFrame(), flip),
            }
        })
        .collect();
    // The same left-to-right, top-to-bottom order Windows uses, so
    // move-to-next-display behaves the same way on both.
    out.sort_by_key(|s| (s.work_area.x, s.work_area.y));
    out
}

fn flipped(rect: CGRect, flip: f64) -> Rect {
    Rect {
        x: rect.origin.x.round() as i32,
        y: (flip - rect.origin.y - rect.size.height).round() as i32,
        width: rect.size.width.round() as i32,
        height: rect.size.height.round() as i32,
    }
}

fn work_area_for(frame: Rect, screens: &[Screen]) -> Option<Rect> {
    let cx = frame.x + frame.width / 2;
    let cy = frame.y + frame.height / 2;
    screens
        .iter()
        .find(|s| {
            cx >= s.frame.x
                && cx < s.frame.x + s.frame.width
                && cy >= s.frame.y
                && cy < s.frame.y + s.frame.height
        })
        .or_else(|| screens.first())
        .map(|s| s.work_area)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEFT: Screen = Screen {
        frame: Rect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        },
        work_area: Rect {
            x: 0,
            y: 25,
            width: 1920,
            height: 1000,
        },
    };
    const RIGHT: Screen = Screen {
        frame: Rect {
            x: 1920,
            y: 0,
            width: 1280,
            height: 800,
        },
        work_area: Rect {
            x: 1920,
            y: 0,
            width: 1280,
            height: 800,
        },
    };

    #[test]
    fn flips_the_primary_screen_to_a_top_left_origin() {
        let primary = CGRect::new(CGPoint::new(0.0, 0.0), CGSize::new(1920.0, 1080.0));
        assert_eq!(
            flipped(primary, 1080.0),
            Rect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080
            }
        );
    }

    /// A menu bar takes the top of the screen, which in AppKit's upward space is
    /// height lost from the top of the visible frame, not an origin change.
    #[test]
    fn flips_a_visible_frame_below_the_menu_bar() {
        let visible = CGRect::new(CGPoint::new(0.0, 0.0), CGSize::new(1920.0, 1055.0));
        assert_eq!(
            flipped(visible, 1080.0),
            Rect {
                x: 0,
                y: 25,
                width: 1920,
                height: 1055
            }
        );
    }

    /// A screen sitting above the primary one has a positive AppKit origin and a
    /// negative Accessibility one.
    #[test]
    fn flips_a_screen_above_the_primary_one() {
        let above = CGRect::new(CGPoint::new(0.0, 1080.0), CGSize::new(1280.0, 800.0));
        assert_eq!(
            flipped(above, 1080.0),
            Rect {
                x: 0,
                y: -800,
                width: 1280,
                height: 800
            }
        );
    }

    #[test]
    fn picks_the_screen_the_window_is_centred_on() {
        let frame = Rect {
            x: 2000,
            y: 100,
            width: 400,
            height: 300,
        };
        assert_eq!(work_area_for(frame, &[LEFT, RIGHT]), Some(RIGHT.work_area));
    }

    #[test]
    fn falls_back_to_the_first_screen_when_the_centre_is_nowhere() {
        let frame = Rect {
            x: -5000,
            y: -5000,
            width: 400,
            height: 300,
        };
        assert_eq!(work_area_for(frame, &[LEFT, RIGHT]), Some(LEFT.work_area));
    }

    #[test]
    fn has_no_work_area_without_a_screen() {
        assert_eq!(work_area_for(LEFT.frame, &[]), None);
    }
}
