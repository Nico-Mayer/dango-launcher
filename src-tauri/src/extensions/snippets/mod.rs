//! Snippets and quicklinks: the user's own named templates.

mod extension;
mod store;

pub use extension::{extension_id, OpenUrl, SnippetsExtension, QUICKLINKS_ID, SNIPPETS_ID};
pub use store::{Kind, Record, RecordError, Records};
