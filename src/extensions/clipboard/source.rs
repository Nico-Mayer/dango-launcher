//! The clipboard itself, through `clipboard-rs`, and the one thing it cannot
//! tell us.
//!
//! The crate gives an event-driven watcher, the pasteboard type list, and text
//! and image access on both platforms, so none of that is ours to maintain. It
//! does not say which application did the copying, and that is exactly what the
//! exclusion list needs, so attribution stays behind a platform trait.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use clipboard_rs::common::RustImage;
use clipboard_rs::{Clipboard, ClipboardContent, ClipboardContext, ContentFormat, RustImageData};

use super::history::Content;
use crate::platform;
use crate::platform::clipboard_busy as busy;

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
    /// The type identifiers currently on the clipboard, or `None` when they
    /// cannot be read. Asked before any content is read, so excluded content
    /// never enters memory.
    fn formats(&self) -> Option<Vec<String>>;
    /// The raw bytes of one named format, for flags rather than content.
    fn buffer(&self, _format: &str) -> Option<Vec<u8>> {
        None
    }
    fn text(&self) -> Option<String>;
    /// The contents as encoded PNG bytes, when there is an image.
    fn image(&self) -> Option<Vec<u8>>;
    /// Paths of files copied in a file manager, or `None` when there are none.
    fn files(&self) -> Option<Vec<String>>;
    fn set_text(&self, text: &str);
    fn set_image(&self, png: &[u8]);
    fn set_files(&self, paths: &[String]);
    /// An image file as PNG and as the file itself, in one write, so an app
    /// takes whichever it understands.
    fn set_image_file(&self, _png: &[u8], _path: &Path) -> Result<(), String> {
        Err("this clipboard can't hold an image file".into())
    }

    /// Everything on the clipboard that `put_back` can restore.
    fn snapshot(&self) -> Snapshot {
        let text = self.text().filter(|text| !text.is_empty());
        let image = if text.is_none() { self.image() } else { None };
        Snapshot {
            text,
            image,
            ..Snapshot::default()
        }
    }

    fn put_back(&self, snapshot: &Snapshot) {
        if let Some(text) = &snapshot.text {
            self.set_text(text);
        } else if let Some(png) = &snapshot.image {
            self.set_image(png);
        }
    }
}

/// What the clipboard held, as far as Dango can put it back.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub text: Option<String>,
    pub html: Option<String>,
    pub rtf: Option<String>,
    pub files: Vec<String>,
    /// Encoded PNG bytes.
    pub image: Option<Vec<u8>>,
}

impl Snapshot {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// The image goes back only without text or files: on Windows the crate
    /// empties the clipboard whenever it writes an image, so the two cannot be
    /// restored together.
    fn keeps_image(&self) -> bool {
        self.text.is_none() && self.files.is_empty()
    }

    /// What the clipboard history reads once this is put back.
    pub fn content(&self) -> Option<Content> {
        if !self.files.is_empty() {
            return Some(Content::Files(self.files.clone()));
        }
        if let Some(text) = &self.text {
            return Some(Content::Text(text.clone()));
        }
        self.image
            .clone()
            .filter(|_| self.keeps_image())
            .map(Content::Image)
    }
}

pub struct CrateClipboard {
    context: ClipboardContext,
}

impl CrateClipboard {
    pub fn new() -> Option<Arc<Self>> {
        match ClipboardContext::new() {
            Ok(context) => Some(Arc::new(Self { context })),
            Err(error) => {
                crate::log::append(&format!("no clipboard available: {error}"));
                None
            }
        }
    }
}

impl ClipboardSource for CrateClipboard {
    fn formats(&self) -> Option<Vec<String>> {
        until_free(
            || match self.context.available_formats() {
                Ok(formats) if formats.is_empty() && busy::holds_anything() => Err(()),
                Ok(formats) => Ok(formats),
                Err(_) => Err(()),
            },
            busy::holds_anything,
        )
        .ok()
    }

    fn buffer(&self, format: &str) -> Option<Vec<u8>> {
        until_free(|| self.context.get_buffer(format), busy::holds_anything).ok()
    }

    fn text(&self) -> Option<String> {
        until_free(|| self.context.get_text(), busy::holds_text)
            .ok()
            .map(platform::text_from_clipboard)
    }

    fn image(&self) -> Option<Vec<u8>> {
        let image = until_free(|| self.context.get_image(), busy::holds_image).ok()?;
        Some(image.to_png().ok()?.get_bytes().to_vec())
    }

    fn files(&self) -> Option<Vec<String>> {
        until_free(|| self.context.get_files(), busy::holds_anything)
            .ok()
            .filter(|paths| !paths.is_empty())
    }

    fn set_files(&self, paths: &[String]) {
        let written = until_free(|| self.context.set_files(paths.to_vec()), busy::possible);
        if let Err(error) = written {
            crate::log::append(&format!(
                "could not put files back on the clipboard: {error}"
            ));
        }
    }

    fn set_image_file(&self, png: &[u8], path: &Path) -> Result<(), String> {
        let image = platform::ImageFile::new(png, path).map_err(|error| error.to_string())?;
        until_free(|| image.write(&self.context), busy::possible).map_err(|error| error.to_string())
    }

    fn set_text(&self, text: &str) {
        let text = platform::text_to_clipboard(text);
        let written = until_free(|| self.context.set_text(text.to_string()), busy::possible);
        if let Err(error) = written {
            crate::log::append(&format!(
                "could not put text back on the clipboard: {error}"
            ));
        }
    }

    fn set_image(&self, png: &[u8]) {
        let mut decoded = match RustImageData::from_bytes(png) {
            Ok(image) => Some(image),
            Err(error) => {
                crate::log::append(&format!("could not decode a stored image: {error}"));
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
            crate::log::append(&format!(
                "could not put an image back on the clipboard: {error}"
            ));
        }
    }

    fn snapshot(&self) -> Snapshot {
        const FORMATS: [ContentFormat; 4] = [
            ContentFormat::Text,
            ContentFormat::Html,
            ContentFormat::Rtf,
            ContentFormat::Files,
        ];
        let contents = until_free(
            || match self.context.get(&FORMATS) {
                Ok(contents) if contents.is_empty() && busy::holds_anything() => Err(()),
                Ok(contents) => Ok(contents),
                Err(_) => Err(()),
            },
            busy::holds_anything,
        )
        .unwrap_or_default();

        let mut snapshot = Snapshot::default();
        for content in contents {
            match content {
                ClipboardContent::Text(text) if !text.is_empty() => {
                    snapshot.text = Some(platform::text_from_clipboard(text))
                }
                ClipboardContent::Html(html) => snapshot.html = Some(html),
                ClipboardContent::Rtf(rtf) => snapshot.rtf = Some(rtf),
                ClipboardContent::Files(files) => snapshot.files = files,
                _ => {}
            }
        }
        // Decoding and re-encoding an image is the slow part, so it is only
        // read when it would be put back.
        if snapshot.keeps_image() {
            snapshot.image = self.image();
        }
        snapshot
    }

    fn put_back(&self, snapshot: &Snapshot) {
        if snapshot.keeps_image() {
            if let Some(png) = &snapshot.image {
                self.set_image(png);
                return;
            }
        }
        let mut first = Some(crate_contents(snapshot));
        if first.as_ref().is_some_and(Vec::is_empty) {
            return;
        }
        let written = until_free(
            || {
                let contents = first.take().unwrap_or_else(|| crate_contents(snapshot));
                self.context.set(contents)
            },
            busy::possible,
        );
        if let Err(error) = written {
            crate::log::append(&format!("could not put the clipboard back: {error}"));
        }
    }
}

fn crate_contents(snapshot: &Snapshot) -> Vec<ClipboardContent> {
    let mut contents = Vec::new();
    if let Some(text) = &snapshot.text {
        contents.push(ClipboardContent::Text(
            platform::text_to_clipboard(text).into_owned(),
        ));
    }
    if let Some(html) = &snapshot.html {
        contents.push(ClipboardContent::Html(html.clone()));
    }
    if let Some(rtf) = &snapshot.rtf {
        contents.push(ClipboardContent::Rtf(rtf.clone()));
    }
    if !snapshot.files.is_empty() {
        contents.push(ClipboardContent::Files(snapshot.files.clone()));
    }
    contents
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_source_without_image_files_refuses_rather_than_pretends() {
        struct TextOnly;
        impl ClipboardSource for TextOnly {
            fn formats(&self) -> Option<Vec<String>> {
                None
            }
            fn text(&self) -> Option<String> {
                None
            }
            fn image(&self) -> Option<Vec<u8>> {
                None
            }
            fn files(&self) -> Option<Vec<String>> {
                None
            }
            fn set_text(&self, _text: &str) {}
            fn set_image(&self, _png: &[u8]) {}
            fn set_files(&self, _paths: &[String]) {}
        }
        assert!(TextOnly
            .set_image_file(b"png", Path::new("/shots/a.png"))
            .is_err());
    }

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
                .unwrap_or_default()
                .iter()
                .any(|f| f.to_ascii_lowercase().contains("text")),
            "the format list is what the privacy check reads"
        );
    }

    #[test]
    #[ignore]
    fn an_image_file_reads_back_as_image_and_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("shot.png");
        image::RgbaImage::from_pixel(4, 3, image::Rgba([200, 30, 30, 255]))
            .save(&path)
            .unwrap();
        let png = std::fs::read(&path).unwrap();
        let clipboard = CrateClipboard::new().unwrap();
        clipboard.set_image_file(&png, &path).unwrap();
        let files = clipboard.files().expect("the file is on the clipboard");
        let canonical = |path: &Path| std::fs::canonicalize(path).unwrap();
        assert_eq!(
            files
                .iter()
                .map(|file| canonical(Path::new(file)))
                .collect::<Vec<_>>(),
            [canonical(&path)]
        );
        let image = clipboard.image().expect("the image is on the clipboard");
        assert_eq!(image_dimensions(&image), Some((4, 3)));
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
    fn files_survive_a_round_trip() {
        let dir = std::env::temp_dir().join("dango-files-round-trip");
        std::fs::create_dir_all(&dir).unwrap();
        let paths: Vec<String> = ["one.txt", "two.txt"]
            .iter()
            .map(|name| {
                let path = dir.join(name);
                std::fs::write(&path, name).unwrap();
                path.to_string_lossy().into_owned()
            })
            .collect();
        let clipboard = CrateClipboard::new().unwrap();
        clipboard.set_files(&paths);
        assert_eq!(clipboard.files(), Some(paths.clone()));
        assert_eq!(clipboard.snapshot().content(), Some(Content::Files(paths)));
        clipboard.set_text("dango after files");
        assert_eq!(clipboard.files(), None);
    }

    #[test]
    #[ignore]
    fn an_image_survives_a_round_trip() {
        let png = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/icons/32x32.png")).unwrap();
        let clipboard = CrateClipboard::new().unwrap();
        clipboard.set_image(&png);
        let read = clipboard.image().expect("an image must come back");
        assert!(!read.is_empty());
        assert!(clipboard.text().is_none(), "an image is not text");
    }
}
