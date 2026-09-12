//! The recorded history: what was copied, in what order, and what gets thrown
//! away to keep it bounded.
//!
//! Text lives in the row. An image lives as a PNG beside the database, because
//! the webview can load a file through the asset protocol but cannot render a
//! blob without a round trip, and because a blob would make the size ceiling a
//! `VACUUM` problem in the file that also holds preferences and frecency.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::store::Store;

/// What was copied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Content {
    Text(String),
    /// Encoded PNG bytes, not yet written anywhere.
    Image(Vec<u8>),
}

impl Content {
    fn bytes(&self) -> usize {
        match self {
            Content::Text(text) => text.len(),
            Content::Image(bytes) => bytes.len(),
        }
    }

    /// Identifies content so the same thing copied twice reorders rather than
    /// duplicates. A hash rather than the content itself, because the column is
    /// unique-indexed and an image is megabytes.
    fn fingerprint(&self) -> String {
        let mut hasher = DefaultHasher::new();
        match self {
            Content::Text(text) => {
                "text".hash(&mut hasher);
                text.hash(&mut hasher);
            }
            Content::Image(bytes) => {
                "image".hash(&mut hasher);
                bytes.hash(&mut hasher);
            }
        }
        format!("{:016x}", hasher.finish())
    }
}

/// One entry as the history hands it back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub id: String,
    pub kind: Kind,
    /// The copied text, or `None` for an image.
    pub text: Option<String>,
    /// Where the image is, or `None` for text.
    pub path: Option<String>,
    pub copied_at: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Text,
    Image,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Kind::Text => "text",
            Kind::Image => "image",
        }
    }

    fn parse(raw: &str) -> Self {
        if raw == "image" {
            Kind::Image
        } else {
            Kind::Text
        }
    }
}

/// What the history is allowed to grow to.
#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    pub entries: usize,
    pub entry_bytes: usize,
    pub total_bytes: usize,
}

impl Default for Bounds {
    fn default() -> Self {
        Self {
            entries: 500,
            entry_bytes: 16 * 1024 * 1024,
            total_bytes: 256 * 1024 * 1024,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    #[error("that item is too large to keep")]
    TooLarge,
    #[error("that entry is no longer there")]
    Gone,
    #[error("{0}")]
    Failed(String),
}

impl From<rusqlite::Error> for HistoryError {
    fn from(error: rusqlite::Error) -> Self {
        HistoryError::Failed(error.to_string())
    }
}

pub struct History {
    store: Arc<Store>,
    images: PathBuf,
}

impl History {
    pub fn new(store: Arc<Store>, images: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&images);
        Self { store, images }
    }

    /// Records what was copied and trims the history back inside its bounds.
    /// Already-present content is moved to newest rather than added again.
    pub fn record(&self, content: Content, bounds: Bounds, now: i64) -> Result<(), HistoryError> {
        let bytes = content.bytes();
        // An entry that cannot fit the whole budget is refused rather than
        // recorded: trimming discards oldest first until it fits, and for
        // something larger than the ceiling that only ends with an empty
        // history. Copying one large thing must not cost the user everything
        // else they copied.
        if bytes > bounds.entry_bytes || bytes > bounds.total_bytes {
            return Err(HistoryError::TooLarge);
        }

        let fingerprint = content.fingerprint();
        if self.touch(&fingerprint, now)? {
            return Ok(());
        }

        let id = uuid::Uuid::new_v4().to_string();
        let (kind, text, path) = match &content {
            Content::Text(text) => (Kind::Text, Some(text.clone()), None),
            Content::Image(image) => {
                let path = self.images.join(format!("{id}.png"));
                std::fs::write(&path, image)
                    .map_err(|error| HistoryError::Failed(error.to_string()))?;
                (Kind::Image, None, Some(path.to_string_lossy().into_owned()))
            }
        };

        self.store.with(|c| {
            c.execute(
                "INSERT INTO local_clipboard_history \
                 (entry_id, kind, text, path, bytes, fingerprint, copied_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    id,
                    kind.as_str(),
                    text,
                    path,
                    bytes as i64,
                    fingerprint,
                    now
                ],
            )?;
            Ok(())
        })?;

        self.trim(bounds)
    }

    /// Newest first, which is the order the history is always read in.
    pub fn entries(&self) -> Result<Vec<Entry>, HistoryError> {
        let entries = self.store.with(|c| {
            let mut stmt = c.prepare(
                "SELECT entry_id, kind, text, path, copied_at FROM local_clipboard_history \
                 ORDER BY copied_at DESC, rowid DESC",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(Entry {
                    id: row.get(0)?,
                    kind: Kind::parse(&row.get::<_, String>(1)?),
                    text: row.get(2)?,
                    path: row.get(3)?,
                    copied_at: row.get(4)?,
                })
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })?;
        Ok(entries)
    }

    pub fn get(&self, id: &str) -> Result<Entry, HistoryError> {
        self.entries()?
            .into_iter()
            .find(|entry| entry.id == id)
            .ok_or(HistoryError::Gone)
    }

    /// The content of an entry, ready to go back on the clipboard. An image
    /// whose file has gone takes its own row with it, since the row without the
    /// file is of no use to anyone.
    pub fn content(&self, id: &str) -> Result<Content, HistoryError> {
        let entry = self.get(id)?;
        match entry.kind {
            Kind::Text => Ok(Content::Text(entry.text.unwrap_or_default())),
            Kind::Image => {
                let path = entry.path.clone().ok_or(HistoryError::Gone)?;
                match std::fs::read(&path) {
                    Ok(bytes) => Ok(Content::Image(bytes)),
                    Err(_) => {
                        self.remove(id)?;
                        Err(HistoryError::Gone)
                    }
                }
            }
        }
    }

    pub fn remove(&self, id: &str) -> Result<(), HistoryError> {
        let path = self.store.with(|c| {
            let path = c
                .query_row(
                    "SELECT path FROM local_clipboard_history WHERE entry_id = ?1",
                    [id],
                    |row| row.get::<_, Option<String>>(0),
                )
                .ok()
                .flatten();
            c.execute(
                "DELETE FROM local_clipboard_history WHERE entry_id = ?1",
                [id],
            )?;
            Ok(path)
        })?;
        delete_file(path.as_deref());
        Ok(())
    }

    /// Moves existing content to newest. Answers whether it was there.
    fn touch(&self, fingerprint: &str, now: i64) -> Result<bool, HistoryError> {
        let changed = self.store.with(|c| {
            c.execute(
                "UPDATE local_clipboard_history SET copied_at = ?2 WHERE fingerprint = ?1",
                rusqlite::params![fingerprint, now],
            )
        })?;
        Ok(changed > 0)
    }

    /// Discards oldest first until both bounds hold. The size bound is enforced
    /// after writing because an image has to be encoded before its size is
    /// known; the per-entry ceiling is what keeps that overshoot small.
    fn trim(&self, bounds: Bounds) -> Result<(), HistoryError> {
        let rows: Vec<(String, Option<String>, i64)> = self.store.with(|c| {
            let mut stmt = c.prepare(
                "SELECT entry_id, path, bytes FROM local_clipboard_history \
                 ORDER BY copied_at DESC, rowid DESC",
            )?;
            let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })?;

        let mut running = 0i64;
        let mut doomed = Vec::new();
        for (index, (id, path, bytes)) in rows.into_iter().enumerate() {
            running += bytes;
            if index >= bounds.entries || running > bounds.total_bytes as i64 {
                doomed.push((id, path));
            }
        }

        for (id, path) in doomed {
            self.store.with(|c| {
                c.execute(
                    "DELETE FROM local_clipboard_history WHERE entry_id = ?1",
                    [&id],
                )?;
                Ok(())
            })?;
            delete_file(path.as_deref());
        }
        Ok(())
    }
}

/// Discarding an image has to actually reclaim its space, or the size bound
/// counts what it no longer holds.
fn delete_file(path: Option<&str>) {
    if let Some(path) = path {
        let _ = std::fs::remove_file(Path::new(path));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn history() -> History {
        let dir = std::env::temp_dir().join(format!("dango-clip-{}", uuid::Uuid::new_v4()));
        History::new(Arc::new(Store::in_memory().unwrap()), dir)
    }

    fn text(value: &str) -> Content {
        Content::Text(value.into())
    }

    fn image(size: usize) -> Content {
        Content::Image(vec![7; size])
    }

    /// Distinct content of a given size, for tests about size rather than
    /// identity: identical bytes would deduplicate into one entry.
    fn distinct_image(fill: u8, size: usize) -> Content {
        Content::Image(vec![fill; size])
    }

    fn titles(history: &History) -> Vec<String> {
        history
            .entries()
            .unwrap()
            .into_iter()
            .map(|e| e.text.unwrap_or_else(|| "<image>".into()))
            .collect()
    }

    #[test]
    fn a_text_entry_is_recorded_and_read_back() {
        let history = history();
        history.record(text("hello"), Bounds::default(), 1).unwrap();
        let entries = history.entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].kind, Kind::Text);
        assert_eq!(entries[0].text.as_deref(), Some("hello"));
        assert!(entries[0].path.is_none());
    }

    #[test]
    fn entries_come_back_newest_first() {
        let history = history();
        history.record(text("one"), Bounds::default(), 1).unwrap();
        history.record(text("two"), Bounds::default(), 2).unwrap();
        history.record(text("three"), Bounds::default(), 3).unwrap();
        assert_eq!(titles(&history), ["three", "two", "one"]);
    }

    #[test]
    fn an_image_entry_is_a_file_and_a_row() {
        let history = history();
        history.record(image(64), Bounds::default(), 1).unwrap();
        let entries = history.entries().unwrap();
        assert_eq!(entries[0].kind, Kind::Image);
        let path = entries[0].path.clone().expect("an image has a path");
        assert!(Path::new(&path).exists(), "the row must point at a file");
        assert!(entries[0].text.is_none());
    }

    #[test]
    fn copying_the_same_thing_twice_reorders_instead_of_duplicating() {
        let history = history();
        history.record(text("one"), Bounds::default(), 1).unwrap();
        history.record(text("two"), Bounds::default(), 2).unwrap();
        history.record(text("one"), Bounds::default(), 3).unwrap();

        assert_eq!(titles(&history), ["one", "two"]);
        assert_eq!(history.entries().unwrap().len(), 2);
    }

    #[test]
    fn the_entry_count_bound_discards_oldest_first() {
        let history = history();
        let bounds = Bounds {
            entries: 3,
            ..Bounds::default()
        };
        for (index, value) in ["a", "b", "c", "d", "e"].iter().enumerate() {
            history
                .record(text(value), bounds, index as i64 + 1)
                .unwrap();
        }
        assert_eq!(titles(&history), ["e", "d", "c"]);
    }

    #[test]
    fn the_total_size_bound_discards_oldest_first() {
        let history = history();
        let bounds = Bounds {
            total_bytes: 250,
            ..Bounds::default()
        };
        for index in 0..4u8 {
            history
                .record(distinct_image(index, 100), bounds, index as i64 + 1)
                .unwrap();
        }
        assert_eq!(
            history.entries().unwrap().len(),
            2,
            "only what fits in the ceiling is kept"
        );
    }

    #[test]
    fn an_item_over_the_per_entry_ceiling_is_refused() {
        let history = history();
        history.record(text("kept"), Bounds::default(), 1).unwrap();
        let bounds = Bounds {
            entry_bytes: 10,
            ..Bounds::default()
        };

        let error = history.record(image(4096), bounds, 2).unwrap_err();
        assert!(matches!(error, HistoryError::TooLarge));
        assert_eq!(
            titles(&history),
            ["kept"],
            "refusing an item must leave the history alone"
        );
    }

    #[test]
    fn an_item_larger_than_the_whole_budget_is_refused_rather_than_emptying_it() {
        let history = history();
        let bounds = Bounds {
            total_bytes: 500,
            ..Bounds::default()
        };
        history.record(text("worth keeping"), bounds, 1).unwrap();

        let error = history
            .record(distinct_image(9, 4096), bounds, 2)
            .unwrap_err();
        assert!(matches!(error, HistoryError::TooLarge));
        assert_eq!(
            titles(&history),
            ["worth keeping"],
            "one oversized copy must not cost the user the rest of the history"
        );
    }

    #[test]
    fn discarding_an_image_deletes_its_file() {
        let history = history();
        let bounds = Bounds {
            entries: 1,
            ..Bounds::default()
        };
        history.record(distinct_image(1, 32), bounds, 1).unwrap();
        let path = history.entries().unwrap()[0].path.clone().unwrap();
        assert!(Path::new(&path).exists());

        history.record(distinct_image(2, 32), bounds, 2).unwrap();
        assert!(
            !Path::new(&path).exists(),
            "the discarded image must not stay on disk"
        );
    }

    #[test]
    fn removing_an_entry_deletes_its_file_too() {
        let history = history();
        history.record(image(32), Bounds::default(), 1).unwrap();
        let entry = history.entries().unwrap().remove(0);
        let path = entry.path.clone().unwrap();

        history.remove(&entry.id).unwrap();
        assert!(history.entries().unwrap().is_empty());
        assert!(!Path::new(&path).exists());
    }

    #[test]
    fn an_entry_whose_file_has_gone_is_reported_and_removed() {
        let history = history();
        history.record(image(32), Bounds::default(), 1).unwrap();
        let entry = history.entries().unwrap().remove(0);
        std::fs::remove_file(entry.path.as_deref().unwrap()).unwrap();

        let error = history.content(&entry.id).unwrap_err();
        assert!(matches!(error, HistoryError::Gone));
        assert!(
            history.entries().unwrap().is_empty(),
            "a row without its file is of no use and goes"
        );
    }

    #[test]
    fn content_comes_back_as_it_went_in() {
        let history = history();
        history.record(text("hello"), Bounds::default(), 1).unwrap();
        history.record(image(16), Bounds::default(), 2).unwrap();
        let entries = history.entries().unwrap();

        assert_eq!(history.content(&entries[1].id).unwrap(), text("hello"));
        assert_eq!(history.content(&entries[0].id).unwrap(), image(16));
    }

    #[test]
    fn asking_for_an_entry_that_is_not_there_reports_it() {
        let history = history();
        assert!(matches!(
            history.content("nope").unwrap_err(),
            HistoryError::Gone
        ));
    }
}
