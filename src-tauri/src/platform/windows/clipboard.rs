//! The Windows clipboard.
//!
//! Not implemented yet. The design calls for spiking the sequence number, the
//! exclusion formats, and the clipboard owner against a real Windows desktop
//! before building on them, and this project cannot compile or run Windows code
//! from the machine development happens on.
//!
//! Until that spike runs, the sequence never moves, so the watcher sees no
//! changes and records nothing. An empty history is the right failure here: the
//! alternative is recording without being able to check the exclusions, and
//! this is the one feature where that costs a password.

use crate::extensions::clipboard::ClipboardSource;

pub struct WindowsClipboard;

impl ClipboardSource for WindowsClipboard {
    fn sequence(&self) -> i64 {
        0
    }

    fn is_excluded(&self) -> bool {
        true
    }

    fn candidate_applications(&self) -> Vec<String> {
        Vec::new()
    }

    fn text(&self) -> Option<String> {
        None
    }

    fn image(&self) -> Option<Vec<u8>> {
        None
    }

    fn set_text(&self, _text: &str) {}

    fn set_image(&self, _png: &[u8]) {}
}
