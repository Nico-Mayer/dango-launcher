//! Which application did the copying, on Windows.
//!
//! The clipboard itself is `clipboard-rs`, which works here too. Only the
//! owner lookup is platform code, and it is not written yet: the design calls
//! for spiking it against a real Windows desktop, and this project cannot
//! compile or run Windows code from the machine development happens on.
//!
//! Until then it claims every application is a candidate, so anything on the
//! exclusion list keeps its clipboard out. That errs towards recording less,
//! which is the right direction for the one feature where the other mistake
//! costs a password.

use crate::extensions::clipboard::Attribution;

pub struct WindowsAttribution;

impl Attribution for WindowsAttribution {
    fn candidate_applications(&self) -> Vec<String> {
        Vec::new()
    }
}
