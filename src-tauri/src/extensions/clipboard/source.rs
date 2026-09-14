//! The clipboard itself, through `clipboard-rs`, and the one thing it cannot
//! tell us.
//!
//! The crate gives an event-driven watcher, the pasteboard type list, and text
//! and image access on both platforms, so none of that is ours to maintain. It
//! does not say which application did the copying, and that is exactly what the
//! exclusion list needs, so attribution stays behind a platform trait.

use std::sync::Arc;
use std::time::{Duration, Instant};

use clipboard_rs::common::RustImage;
use clipboard_rs::{Clipboard, ClipboardContext, RustImageData};

/// How long to wait for another process to let go of the clipboard.
///
/// Windows opens the clipboard to one window at a time. A paste target reading
/// what it was sent, or the system's clipboard history looking at a change,
/// holds it for a few milliseconds, and a read or write that lands in that
/// window fails with access denied. The crate retries only with a scheduler
/// yield, which is not long enough, so the wait happens here. Without it a
/// restore is skipped or lost and the user's clipboard ends up holding what
/// Dango pasted.
const BUSY_WAIT: Duration = Duration::from_millis(250);
const BUSY_POLL: Duration = Duration::from_millis(2);

/// The pixel size of encoded image bytes, or `None` when they cannot be
/// decoded. Used to recognise Dango's own image writes, which come back from
/// the pasteboard re-encoded rather than byte for byte.
pub fn image_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    RustImageData::from_bytes(bytes)
        .ok()
        .map(|image| image.get_size())
}

/// Which applications could have put the current contents there.
///
/// Windows can name the owner exactly. macOS cannot, and answers with every
/// application that was frontmost around the change, because the spike showed
/// that asking at the instant of noticing attributes a password to whatever the
/// user switched to.
pub trait Attribution: Send + Sync {
    fn candidate_applications(&self) -> Vec<String>;
    /// Called when a change arrives, before anything is read. A platform that
    /// can tell the copying application is still busy finishing its copy
    /// waits here for it.
    fn wait_for_copy_to_finish(&self) {}
}

/// Reading and writing the clipboard. A trait so the watcher can be tested
/// without a real one, not because there is a second implementation.
pub trait ClipboardSource: Send + Sync {
    /// The type identifiers currently on the clipboard. Asked before any
    /// content is read, so excluded content never enters memory.
    fn formats(&self) -> Vec<String>;
    fn text(&self) -> Option<String>;
    /// The contents as encoded PNG bytes, when there is an image.
    fn image(&self) -> Option<Vec<u8>>;
    fn set_text(&self, text: &str);
    fn set_image(&self, png: &[u8]);
}

pub struct CrateClipboard {
    context: ClipboardContext,
}

impl CrateClipboard {
    pub fn new() -> Option<Arc<Self>> {
        match ClipboardContext::new() {
            Ok(context) => Some(Arc::new(Self { context })),
            Err(error) => {
                eprintln!("[dango] no clipboard available: {error}");
                None
            }
        }
    }
}

impl ClipboardSource for CrateClipboard {
    fn formats(&self) -> Vec<String> {
        until_free(
            || {
                let formats = self.context.available_formats().unwrap_or_default();
                if formats.is_empty() && busy::holds_anything() {
                    Err(())
                } else {
                    Ok(formats)
                }
            },
            busy::holds_anything,
        )
        .unwrap_or_default()
    }

    fn text(&self) -> Option<String> {
        until_free(|| self.context.get_text(), busy::holds_text).ok()
    }

    fn image(&self) -> Option<Vec<u8>> {
        let image = until_free(|| self.context.get_image(), busy::holds_image).ok()?;
        Some(image.to_png().ok()?.get_bytes().to_vec())
    }

    fn set_text(&self, text: &str) {
        let written = until_free(|| self.context.set_text(text.to_string()), busy::possible);
        if let Err(error) = written {
            eprintln!("[dango] could not put text back on the clipboard: {error}");
        }
    }

    fn set_image(&self, png: &[u8]) {
        let mut decoded = match RustImageData::from_bytes(png) {
            Ok(image) => Some(image),
            Err(error) => {
                eprintln!("[dango] could not decode a stored image: {error}");
                return;
            }
        };
        let written = until_free(
            || {
                let image = match decoded.take() {
                    Some(image) => image,
                    None => RustImageData::from_bytes(png)?,
                };
                self.context.set_image(image)
            },
            busy::possible,
        );
        if let Err(error) = written {
            eprintln!("[dango] could not put an image back on the clipboard: {error}");
        }
    }
}

/// Retries `attempt` while `worth_waiting` says a failure is the clipboard
/// being busy rather than the content being absent, up to `BUSY_WAIT`.
fn until_free<T, E>(
    mut attempt: impl FnMut() -> Result<T, E>,
    worth_waiting: impl Fn() -> bool,
) -> Result<T, E> {
    let deadline = Instant::now() + BUSY_WAIT;
    loop {
        match attempt() {
            Ok(value) => return Ok(value),
            Err(_) if Instant::now() < deadline && worth_waiting() => {
                std::thread::sleep(BUSY_POLL);
            }
            Err(error) => return Err(error),
        }
    }
}

/// Whether a failed clipboard call can be the clipboard being held by another
/// window. These ask without opening the clipboard, so they answer while it
/// is held.
#[cfg(target_os = "windows")]
mod busy {
    use windows_sys::Win32::System::DataExchange::{
        CountClipboardFormats, IsClipboardFormatAvailable,
    };
    use windows_sys::Win32::System::Ole::{CF_BITMAP, CF_DIB, CF_DIBV5, CF_UNICODETEXT};

    pub fn possible() -> bool {
        true
    }

    pub fn holds_anything() -> bool {
        unsafe { CountClipboardFormats() > 0 }
    }

    pub fn holds_text() -> bool {
        holds(&[CF_UNICODETEXT])
    }

    pub fn holds_image() -> bool {
        holds(&[CF_BITMAP, CF_DIB, CF_DIBV5])
    }

    fn holds(formats: &[u16]) -> bool {
        formats
            .iter()
            .any(|format| unsafe { IsClipboardFormatAvailable(u32::from(*format)) } != 0)
    }
}

/// The macOS pasteboard has no exclusive open, so a failure is never "busy".
#[cfg(not(target_os = "windows"))]
mod busy {
    pub fn possible() -> bool {
        false
    }

    pub fn holds_anything() -> bool {
        false
    }

    pub fn holds_text() -> bool {
        false
    }

    pub fn holds_image() -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs a real clipboard, so it runs on a desktop rather than in CI, and
    /// one test at a time since they share it:
    /// `cargo test -- --ignored --test-threads=1`.
    #[test]
    #[ignore]
    fn text_survives_a_round_trip() {
        let clipboard = CrateClipboard::new().unwrap();
        clipboard.set_text("dango round trip");
        assert_eq!(clipboard.text().as_deref(), Some("dango round trip"));
        assert!(
            clipboard
                .formats()
                .iter()
                .any(|f| f.to_ascii_lowercase().contains("text")),
            "the format list is what the privacy check reads"
        );
    }

    /// Another process holding the clipboard open, as a target does while it
    /// reads a paste and as the system's clipboard history does after every
    /// change. The holder must be another process and must open with a real
    /// window: an open with no window, or from this process, does not block.
    #[cfg(target_os = "windows")]
    #[test]
    #[ignore]
    fn a_busy_clipboard_is_waited_for() {
        use std::io::{BufRead, BufReader};
        use std::process::{Command, Stdio};

        let clipboard = CrateClipboard::new().unwrap();
        clipboard.set_text("dango before busy");
        let mut holder = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                concat!(
                    "Add-Type -AssemblyName System.Windows.Forms;",
                    "Add-Type -Name C -Namespace W -MemberDefinition '",
                    "[DllImport(\"user32.dll\")] public static extern bool OpenClipboard(IntPtr h);",
                    "[DllImport(\"user32.dll\")] public static extern bool CloseClipboard();';",
                    "$form = New-Object System.Windows.Forms.Form;",
                    "if (-not [W.C]::OpenClipboard($form.Handle)) { Write-Output failed; exit 1 };",
                    "Write-Output held; [Console]::Out.Flush();",
                    "Start-Sleep -Milliseconds 60; [W.C]::CloseClipboard() | Out-Null"
                ),
            ])
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut line = String::new();
        BufReader::new(holder.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        assert_eq!(line.trim(), "held");

        let read = clipboard.text();
        clipboard.set_text("dango during busy");
        holder.wait().unwrap();
        assert_eq!(read.as_deref(), Some("dango before busy"));
        assert_eq!(clipboard.text().as_deref(), Some("dango during busy"));
    }

    #[test]
    #[ignore]
    fn an_image_survives_a_round_trip() {
        let png = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../static/favicon.png"
        ))
        .unwrap();
        let clipboard = CrateClipboard::new().unwrap();
        clipboard.set_image(&png);
        let read = clipboard.image().expect("an image must come back");
        assert!(!read.is_empty());
        assert!(clipboard.text().is_none(), "an image is not text");
    }
}
