//! The Windows side of the `system` commands.
//!
//! Not implemented yet. The design calls for spiking these against a real
//! Windows desktop before building on them, and this project cannot compile or
//! run Windows code from the machine development happens on. Until that spike
//! runs, every command reports plainly that it does not work here rather than
//! appearing to do nothing.

use super::{RunningApp, SystemControl, SystemError};

pub struct WindowsSystemControl;

impl SystemControl for WindowsSystemControl {
    fn lock(&self) -> Result<(), SystemError> {
        Err(SystemError::Unsupported("locking the screen"))
    }

    fn sleep(&self) -> Result<(), SystemError> {
        Err(SystemError::Unsupported("sleep"))
    }

    fn trash_count(&self) -> Result<usize, SystemError> {
        Err(SystemError::Unsupported("the recycle bin"))
    }

    fn empty_trash(&self) -> Result<(), SystemError> {
        Err(SystemError::Unsupported("emptying the recycle bin"))
    }

    fn running_apps(&self) -> Vec<RunningApp> {
        Vec::new()
    }

    fn quit(&self, _app_id: &str) -> Result<(), SystemError> {
        Err(SystemError::Unsupported("quitting an application"))
    }
}
