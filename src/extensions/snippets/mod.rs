//! Snippets and quicklinks: the user's own named templates.

mod extension;
mod favicons;
mod matcher;
mod service;
mod store;

pub(crate) use extension::manifest;
pub use extension::{
    extension_id, OpenUrl, SnippetsExtension, ACTION_EDIT, QUICKLINKS_ID, SNIPPETS_ID,
};
pub use favicons::{FaviconCache, FaviconService, FaviconsEnabled, HttpFavicons};
pub use matcher::{match_keyword, KeywordEntry};
pub use service::ExcludedApps;
pub use store::{Kind, QuicklinkEntry, Record, RecordError, Records, SnippetEntry};
