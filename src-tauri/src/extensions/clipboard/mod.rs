//! The `clipboard-history` built-in extension.

mod history;
mod watcher;

pub use history::{Bounds, Content, Entry, History, HistoryError, Kind};
pub use watcher::{ClipboardSource, Policy, WatchService, Watcher, POLL_INTERVAL};
