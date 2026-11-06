//! Root items: rows an extension builds now and then, which Dango keeps and
//! matches against every query without running the extension.

use std::path::Path;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use serde::Deserialize;

use crate::protocol::Action;
use crate::search::{Candidate, RootProvider, Source};

use super::manifest::resolve_icon;

/// One row as `rootItems()` returns it.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RootItem {
    id: String,
    title: String,
    icon: Option<String>,
    #[serde(default)]
    keywords: Vec<String>,
    #[serde(default)]
    actions: Vec<Action>,
}

impl RootItem {
    pub fn candidate(self, extension_id: &str, folder: &Path) -> Candidate {
        Candidate {
            extension_id: extension_id.to_owned(),
            id: self.id,
            title: self.title,
            status: None,
            icon: self.icon.map(|icon| resolve_icon(icon, folder)),
            keywords: self.keywords,
            alias: None,
            source: Source::RootItem,
            actions: self.actions,
            match_positions: Vec::new(),
            suggested: false,
            pinned: false,
        }
    }
}

/// The last rows an extension built. Answering a query only reads them.
#[derive(Default)]
pub struct CachedRows(RwLock<Vec<Candidate>>);

impl CachedRows {
    pub fn replace(&self, rows: Vec<Candidate>) {
        *self.0.write().unwrap() = rows;
    }
}

pub struct CachedProvider(pub Arc<CachedRows>);

#[async_trait]
impl RootProvider for CachedProvider {
    async fn items(&self, _query: String) -> Vec<Candidate> {
        self.0 .0.read().unwrap().clone()
    }
}
