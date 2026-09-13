//! Snippets and quicklinks: the user's own named templates.

mod extension;
mod matcher;
mod store;

pub use extension::{extension_id, OpenUrl, SnippetsExtension, QUICKLINKS_ID, SNIPPETS_ID};
pub use matcher::{match_keyword, KeywordEntry};
pub use store::{Kind, Record, RecordError, Records};
