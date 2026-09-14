//! Snippets and quicklinks: the user's own named templates.

mod extension;
mod matcher;
mod service;
mod store;

#[cfg(test)]
pub(crate) use extension::manifest;
pub use extension::{extension_id, OpenUrl, SnippetsExtension, QUICKLINKS_ID, SNIPPETS_ID};
pub use matcher::{match_keyword, KeywordEntry};
pub use service::ExcludedApps;
pub use store::{Kind, Record, RecordError, Records};
