//! Which application did the copying, on Windows.
//!
//! The clipboard itself is `clipboard-rs`. This is the one question it cannot
//! answer, and Windows answers it exactly: the window that last put something
//! on the clipboard stays its owner until the next copy, and that window's
//! process is the copying application. So one name is the right answer here,
//! where macOS has to offer everything that was frontmost around the change.
//!
//! No owner means no candidates, and the content is recorded. A program that
//! opens the clipboard without a window leaves no owner behind; `clip.exe`
//! does, and so does `clipboard-rs` itself. That is the wrong direction for
//! this feature, so it is a stated limitation rather than a guess at who it
//! might have been.

use windows::Win32::System::DataExchange::GetClipboardOwner;

use super::apps::init_com;
use super::system::{identify, owning_pid};
use crate::extensions::clipboard::Attribution;

pub struct WindowsAttribution;

impl Attribution for WindowsAttribution {
    fn candidate_applications(&self) -> Vec<String> {
        unsafe { owner_name() }.into_iter().collect()
    }
}

/// Named the same way the running applications list names things, so the
/// exclusion list can be filled from what the user sees there.
unsafe fn owner_name() -> Option<String> {
    let owner = GetClipboardOwner().ok()?;
    let pid = owning_pid(owner)?;
    init_com();
    identify(owner, pid).map(|identity| identity.name)
}

#[cfg(test)]
mod tests {
    use windows::core::w;
    use windows::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard};
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, HWND_MESSAGE, WINDOW_EX_STYLE, WINDOW_STYLE,
    };

    use super::*;

    /// Needs a desktop session with a clipboard: `cargo test -- --ignored`.
    ///
    /// Emptying the clipboard through a window of our own is how an ordinary
    /// application becomes its owner. The test binary carries no file
    /// description, so it is named by its file name, which is what makes the
    /// expectation computable.
    #[test]
    #[ignore]
    fn the_process_that_copied_is_the_only_candidate() {
        unsafe {
            let window = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                w!("dango owner"),
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                None,
                None,
            )
            .unwrap();
            OpenClipboard(Some(window)).unwrap();
            EmptyClipboard().unwrap();
            CloseClipboard().unwrap();
            let candidates = WindowsAttribution.candidate_applications();
            let _ = DestroyWindow(window);

            let ours = std::env::current_exe().unwrap();
            let ours = ours.file_stem().unwrap().to_string_lossy().into_owned();
            assert_eq!(candidates, [ours]);
        }
    }
}
