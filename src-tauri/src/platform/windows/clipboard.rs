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

use std::time::Duration;

use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::System::DataExchange::GetClipboardOwner;
use windows::Win32::UI::WindowsAndMessaging::{SendMessageTimeoutW, SMTO_ABORTIFHUNG, WM_NULL};

use super::apps::init_com;
use super::system::{identify, owning_pid};
use crate::extensions::clipboard::Attribution;

/// How long the owner gets to answer a no-op message before it counts as
/// busy, and how long to wait between asking again.
const OWNER_REPLY: u32 = 50;
const OWNER_RETRY: Duration = Duration::from_millis(100);
const OWNER_ATTEMPTS: u32 = 15;

pub struct WindowsAttribution;

impl Attribution for WindowsAttribution {
    fn candidate_applications(&self) -> Vec<String> {
        unsafe { owner_name() }.into_iter().collect()
    }

    /// A .NET application copies by announcing delayed-rendered formats and
    /// then, still on the same thread and without pumping messages, flushing
    /// them for real. Reading in that gap asks the thread to render while it
    /// is stuck retrying to open a clipboard we hold, and after a second of
    /// that the copy fails in the copying application with
    /// CLIPBRD_E_CANT_OPEN. So the owner is asked to answer a no-op message
    /// first: once it does, it is back in its message loop and the copy is
    /// complete.
    fn wait_for_copy_to_finish(&self) {
        for _ in 0..OWNER_ATTEMPTS {
            if unsafe { owner_is_idle() } {
                return;
            }
            std::thread::sleep(OWNER_RETRY);
        }
    }
}

unsafe fn owner_is_idle() -> bool {
    let Ok(owner) = GetClipboardOwner() else {
        return true;
    };
    SendMessageTimeoutW(
        owner,
        WM_NULL,
        WPARAM(0),
        LPARAM(0),
        SMTO_ABORTIFHUNG,
        OWNER_REPLY,
        None,
    )
    .0 != 0
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
