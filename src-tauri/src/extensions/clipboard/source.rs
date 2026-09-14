//! The clipboard itself, through `clipboard-rs`, and the one thing it cannot
//! tell us.
//!
//! The crate gives an event-driven watcher, the pasteboard type list, and text
//! and image access on both platforms, so none of that is ours to maintain. It
//! does not say which application did the copying, and that is exactly what the
//! exclusion list needs, so attribution stays behind a platform trait.

use std::sync::Arc;

use clipboard_rs::common::RustImage;
use clipboard_rs::{Clipboard, ClipboardContext, RustImageData};

/// The pixel size of encoded image bytes, or `None` when they cannot be
/// decoded. Used to recognise Dango's own image writes, which come back from
/// the pasteboard re-encoded rather than byte for byte.
pub fn image_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    RustImageData::from_bytes(bytes).ok().map(|image| image.get_size())
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
        self.context.available_formats().unwrap_or_default()
    }

    fn text(&self) -> Option<String> {
        self.context.get_text().ok()
    }

    fn image(&self) -> Option<Vec<u8>> {
        let image = self.context.get_image().ok()?;
        Some(image.to_png().ok()?.get_bytes().to_vec())
    }

    fn set_text(&self, text: &str) {
        if let Err(error) = self.context.set_text(text.to_string()) {
            eprintln!("[dango] could not put text back on the clipboard: {error}");
        }
    }

    fn set_image(&self, png: &[u8]) {
        match RustImageData::from_bytes(png) {
            Ok(image) => {
                if let Err(error) = self.context.set_image(image) {
                    eprintln!("[dango] could not put an image back on the clipboard: {error}");
                }
            }
            Err(error) => eprintln!("[dango] could not decode a stored image: {error}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs a real clipboard, so it runs on a desktop rather than in CI:
    /// `cargo test -- --ignored`.
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
