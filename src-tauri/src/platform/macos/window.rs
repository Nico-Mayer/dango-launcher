//! The macOS half of window management.
//!
//! Not yet implemented. Moving a foreign window on macOS goes through the
//! Accessibility API (`AXFocusedWindow`, then `kAXPositionAttribute` and
//! `kAXSizeAttribute` as `AXValue`s), gated behind the Accessibility permission
//! M3 surfaces. That code has to be written and its symbols checked against the
//! vendored `objc2-application-services` source on a macOS machine, where it can
//! also be compiled and run; it cannot be verified from the Windows machine this
//! was built on. Until then every command reports the gap rather than pretending
//! to work.

use crate::platform::{Placement, Rect, WindowError};

pub struct MacWindowManager;

impl crate::platform::WindowManager for MacWindowManager {
    fn target(&self) -> Result<Placement, WindowError> {
        Err(WindowError::Failed(
            "window management is not yet available on macOS".into(),
        ))
    }

    fn displays(&self) -> Vec<Rect> {
        Vec::new()
    }

    fn place(&self, _frame: Rect) -> Result<(), WindowError> {
        Err(WindowError::Failed(
            "window management is not yet available on macOS".into(),
        ))
    }
}
