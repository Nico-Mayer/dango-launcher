//! The recorded history: what was copied, in what order, and what gets thrown
//! away to keep it bounded.
//!
//! Text lives in the row. An image lives as a PNG beside the database, because
//! Slint loads an image from a path and caches it, and because a blob would
//! make the size ceiling a `VACUUM` problem in the file that also holds
//! preferences and frecency.

use std::collections::HashSet;
use std::hash::Hasher;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use clipboard_rs::common::RustImage;
use clipboard_rs::RustImageData;
use rusqlite::OptionalExtension;
use siphasher::sip::SipHasher13;

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
        let (kind, bytes): (&[u8], &[u8]) = match self {
            Content::Text(text) => (b"text\0", text.as_bytes()),
            Content::Image(bytes) => (b"image\0", bytes),
        };
        let mut hasher = SipHasher13::new();
        hasher.write(kind);
        hasher.write(bytes);
        format!("{:016x}", hasher.finish())
    }
}

/// One entry as the history hands it back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub id: String,
    pub kind: Kind,
    /// The copied text, or `None` for an image. From `entries`, only the
    /// first `LISTED_CHARS` characters.
    pub text: Option<String>,
    /// Where the image is, or `None` for text.
    pub path: Option<String>,
    /// A small copy of the image for listing, when one was made.
    pub thumbnail: Option<String>,
    /// The application it was copied from, when exactly one could have.
    pub source: Option<String>,
    /// An image's width and height in pixels, when it could be decoded.
    pub size: Option<(u32, u32)>,
    /// Of the whole text, for text.
    pub characters: Option<u32>,
    pub lines: Option<u32>,
    pub copied_at: i64,
}

impl Entry {
    /// What the detail pane shows of an image. Never one larger than the
    /// preview, so moving the selection stays cheap; a row from before sizes
    /// were stored falls back to its thumbnail.
    pub fn pane_image(&self) -> Option<String> {
        let path = self.path.as_deref()?;
        match self.size {
            Some(size) if fits(size, PREVIEW_SIZE) => Some(path.to_string()),
            Some(_) => Some(preview_path(path).to_string_lossy().into_owned()),
            None => self.thumbnail.clone(),
        }
    }
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
    #[error("That entry is too large to keep")]
    TooLarge,
    #[error("That entry was already deleted")]
    Gone,
    #[error("Couldn't read the clipboard history")]
    Failed(String),
}

impl From<rusqlite::Error> for HistoryError {
    fn from(error: rusqlite::Error) -> Self {
        crate::log::append(&format!("clipboard history: {error}"));
        HistoryError::Failed(error.to_string())
    }
}

/// Enough of each text for a one-line preview, so listing the history never
/// loads every copied text in full.
pub(super) const LISTED_CHARS: usize = 2000;

/// The columns after the text that `entry` reads. The counts are of the whole
/// text, however little of it is read.
const DETAILS: &str = "path, copied_at, source, width, height, length(text), \
     length(text) - length(replace(text, char(10), '')) + 1";

/// Sharp in a list row and in a grid tile at 200 % scaling.
const THUMBNAIL_SIZE: u32 = 384;

/// Sharp in the detail pane at 200 % scaling, where the image is at most about
/// 370 logical pixels wide.
const PREVIEW_SIZE: u32 = 800;

pub struct History {
    store: Arc<Store>,
    images: PathBuf,
}

impl History {
    pub fn new(store: Arc<Store>, images: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&images);
        let history = Self { store, images };
        history.delete_orphaned_images();
        history
    }

    /// An image whose row is gone, after a failed insert or a database set
    /// aside as corrupt, would otherwise stay on disk outside every bound.
    fn delete_orphaned_images(&self) {
        let paths = self.store.with(|c| {
            let mut stmt =
                c.prepare("SELECT path FROM local_clipboard_history WHERE path IS NOT NULL")?;
            let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
            rows.collect::<rusqlite::Result<HashSet<_>>>()
        });
        let Ok(mut paths) = paths else {
            return;
        };
        let copies: Vec<String> = paths
            .iter()
            .flat_map(|path| [thumbnail_path(path), preview_path(path)])
            .map(|copy| copy.to_string_lossy().into_owned())
            .collect();
        paths.extend(copies);
        let Ok(files) = std::fs::read_dir(&self.images) else {
            return;
        };
        for file in files.flatten() {
            let path = file.path();
            let is_png = path.extension().is_some_and(|ext| ext == "png");
            if is_png && !paths.contains(path.to_string_lossy().as_ref()) {
                let _ = std::fs::remove_file(&path);
            }
        }
    }

    /// Records what was copied and trims the history back inside its bounds.
    /// Already-present content is moved to newest rather than added again.
    pub fn record(
        &self,
        content: Content,
        source: Option<&str>,
        bounds: Bounds,
        now: i64,
    ) -> Result<(), HistoryError> {
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
        if self.touch(&fingerprint, source, now)? {
            return Ok(());
        }

        let id = uuid::Uuid::new_v4().to_string();
        let mut bytes = bytes;
        let mut size = None;
        let (kind, text, path) = match &content {
            Content::Text(text) => (Kind::Text, Some(text.clone()), None),
            Content::Image(image) => {
                let path = self.images.join(format!("{id}.png"));
                std::fs::write(&path, image).map_err(|error| {
                    crate::log::append(&format!("clipboard history image: {error}"));
                    HistoryError::Failed(error.to_string())
                })?;
                let (written, decoded) = write_smaller_copies(image, &path);
                bytes += written;
                size = decoded;
                (Kind::Image, None, Some(path.to_string_lossy().into_owned()))
            }
        };

        self.store.with(|c| {
            c.execute(
                "INSERT INTO local_clipboard_history \
                 (entry_id, kind, text, path, bytes, fingerprint, copied_at, source, width, height) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    id,
                    kind.as_str(),
                    text,
                    path,
                    bytes as i64,
                    fingerprint,
                    now,
                    source,
                    size.map(|(width, _)| width),
                    size.map(|(_, height)| height)
                ],
            )?;
            Ok(())
        })?;

        self.trim(bounds)
    }

    /// Newest first, which is the order the history is always read in.
    pub fn entries(&self) -> Result<Vec<Entry>, HistoryError> {
        let entries = self.store.with(|c| {
            let mut stmt = c.prepare(&format!(
                "SELECT entry_id, kind, substr(text, 1, {LISTED_CHARS}), {DETAILS} \
                 FROM local_clipboard_history ORDER BY copied_at DESC, rowid DESC"
            ))?;
            let rows = stmt.query_map([], entry)?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })?;
        Ok(entries)
    }

    pub fn get(&self, id: &str) -> Result<Entry, HistoryError> {
        let found = self.store.with(|c| {
            c.query_row(
                &format!(
                    "SELECT entry_id, kind, text, {DETAILS} FROM local_clipboard_history \
                     WHERE entry_id = ?1"
                ),
                [id],
                entry,
            )
            .optional()
        })?;
        found.ok_or(HistoryError::Gone)
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

    /// Moves existing content to newest, from its new source. Answers whether
    /// it was there.
    fn touch(
        &self,
        fingerprint: &str,
        source: Option<&str>,
        now: i64,
    ) -> Result<bool, HistoryError> {
        let changed = self.store.with(|c| {
            c.execute(
                "UPDATE local_clipboard_history SET copied_at = ?2, source = ?3 \
                 WHERE fingerprint = ?1",
                rusqlite::params![fingerprint, now, source],
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

fn entry(row: &rusqlite::Row) -> rusqlite::Result<Entry> {
    let path: Option<String> = row.get(3)?;
    let thumbnail = path
        .as_deref()
        .map(thumbnail_path)
        .filter(|thumbnail| thumbnail.exists())
        .map(|thumbnail| thumbnail.to_string_lossy().into_owned());
    Ok(Entry {
        id: row.get(0)?,
        kind: Kind::parse(&row.get::<_, String>(1)?),
        text: row.get(2)?,
        path,
        thumbnail,
        source: row.get(5)?,
        size: row
            .get::<_, Option<u32>>(6)?
            .zip(row.get::<_, Option<u32>>(7)?),
        characters: row.get(8)?,
        lines: row.get(9)?,
        copied_at: row.get(4)?,
    })
}

fn thumbnail_path(image: impl AsRef<Path>) -> PathBuf {
    image.as_ref().with_extension("thumb.png")
}

fn preview_path(image: impl AsRef<Path>) -> PathBuf {
    image.as_ref().with_extension("preview.png")
}

/// Writes the smaller copies of `png` it needs and answers their size in
/// bytes, with the image's size in pixels when it decodes. An image within a
/// copy's size gets no copy: showing it instead is just as cheap.
fn write_smaller_copies(png: &[u8], path: &Path) -> (usize, Option<(u32, u32)>) {
    let image = match RustImageData::from_bytes(png) {
        Ok(image) => image,
        Err(error) => {
            crate::log::append(&format!("clipboard history thumbnail: {error}"));
            return (0, None);
        }
    };
    let size = image.get_size();
    let mut written = 0;
    let preview = (!fits(size, PREVIEW_SIZE))
        .then(|| shrink(&image, PREVIEW_SIZE, &preview_path(path)))
        .and_then(|shrunk| logged(shrunk, "preview"))
        .map(|(bytes, preview)| {
            written += bytes;
            preview
        });
    if !fits(size, THUMBNAIL_SIZE) {
        let from = preview.as_ref().unwrap_or(&image);
        if let Some((bytes, _)) = logged(
            shrink(from, THUMBNAIL_SIZE, &thumbnail_path(path)),
            "thumbnail",
        ) {
            written += bytes;
        }
    }
    (written, Some(size))
}

fn fits((width, height): (u32, u32), within: u32) -> bool {
    width <= within && height <= within
}

fn shrink(
    image: &RustImageData,
    within: u32,
    path: &Path,
) -> clipboard_rs::Result<(usize, RustImageData)> {
    let smaller = image.thumbnail(within, within)?;
    let png = smaller.to_png()?;
    let bytes = png.get_bytes();
    std::fs::write(path, bytes)?;
    Ok((bytes.len(), smaller))
}

fn logged<T>(result: clipboard_rs::Result<T>, copy: &str) -> Option<T> {
    result
        .inspect_err(|error| crate::log::append(&format!("clipboard history {copy}: {error}")))
        .ok()
}

/// Discarding an image has to actually reclaim its space, or the size bound
/// counts what it no longer holds.
fn delete_file(path: Option<&str>) {
    if let Some(path) = path {
        let _ = std::fs::remove_file(Path::new(path));
        let _ = std::fs::remove_file(thumbnail_path(path));
        let _ = std::fs::remove_file(preview_path(path));
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

    fn png(width: u32, height: u32) -> Content {
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut bytes, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer
            .write_image_data(&vec![200; (width * height * 4) as usize])
            .unwrap();
        writer.finish().unwrap();
        Content::Image(bytes)
    }

    fn png_size(path: &str) -> (u32, u32) {
        let decoder =
            png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).unwrap()));
        let info = decoder.read_info().unwrap();
        (info.info().width, info.info().height)
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
        history
            .record(text("hello"), None, Bounds::default(), 1)
            .unwrap();
        let entries = history.entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].kind, Kind::Text);
        assert_eq!(entries[0].text.as_deref(), Some("hello"));
        assert!(entries[0].path.is_none());
    }

    #[test]
    fn entries_come_back_newest_first() {
        let history = history();
        history
            .record(text("one"), None, Bounds::default(), 1)
            .unwrap();
        history
            .record(text("two"), None, Bounds::default(), 2)
            .unwrap();
        history
            .record(text("three"), None, Bounds::default(), 3)
            .unwrap();
        assert_eq!(titles(&history), ["three", "two", "one"]);
    }

    #[test]
    fn an_image_entry_is_a_file_and_a_row() {
        let history = history();
        history
            .record(image(64), None, Bounds::default(), 1)
            .unwrap();
        let entries = history.entries().unwrap();
        assert_eq!(entries[0].kind, Kind::Image);
        let path = entries[0].path.clone().expect("an image has a path");
        assert!(Path::new(&path).exists(), "the row must point at a file");
        assert!(entries[0].text.is_none());
    }

    #[test]
    fn an_image_entry_has_a_thumbnail_that_keeps_its_aspect_ratio() {
        let history = history();
        history
            .record(png(1600, 900), None, Bounds::default(), 1)
            .unwrap();
        let entry = history.entries().unwrap().remove(0);

        let thumbnail = entry.thumbnail.expect("a decodable image has a thumbnail");
        assert!(thumbnail.ends_with(".thumb.png"));
        assert_eq!(png_size(&thumbnail), (384, 216));
    }

    #[test]
    fn an_image_within_the_thumbnail_size_gets_no_thumbnail() {
        let history = history();
        history
            .record(png(384, 20), None, Bounds::default(), 1)
            .unwrap();
        assert!(history.entries().unwrap()[0].thumbnail.is_none());
    }

    #[test]
    fn the_thumbnail_counts_toward_the_size_bound() {
        let history = history();
        let image = png(800, 800);
        let image_bytes = image.bytes() as i64;
        history.record(image, None, Bounds::default(), 1).unwrap();
        let entry = history.entries().unwrap().remove(0);
        let thumbnail_bytes = std::fs::metadata(entry.thumbnail.unwrap()).unwrap().len() as i64;

        let stored: i64 = history
            .store
            .with(|c| {
                c.query_row("SELECT bytes FROM local_clipboard_history", [], |row| {
                    row.get(0)
                })
            })
            .unwrap();
        assert_eq!(stored, image_bytes + thumbnail_bytes);
    }

    fn stored_bytes(history: &History) -> i64 {
        history
            .store
            .with(|c| {
                c.query_row("SELECT bytes FROM local_clipboard_history", [], |row| {
                    row.get(0)
                })
            })
            .unwrap()
    }

    #[test]
    fn a_large_image_gets_a_preview_that_is_counted_and_deleted_with_it() {
        let history = history();
        let image = png(1600, 1000);
        let image_bytes = image.bytes() as u64;
        history.record(image, None, Bounds::default(), 1).unwrap();
        let entry = history.entries().unwrap().remove(0);
        let path = entry.path.clone().unwrap();
        let preview = preview_path(&path);
        let thumbnail = entry.thumbnail.clone().unwrap();

        assert_eq!(png_size(preview.to_str().unwrap()), (800, 500));
        assert_eq!(png_size(&thumbnail), (384, 240));
        let copies = std::fs::metadata(&preview).unwrap().len()
            + std::fs::metadata(&thumbnail).unwrap().len();
        assert_eq!(stored_bytes(&history), (image_bytes + copies) as i64);

        history.remove(&entry.id).unwrap();
        assert!(!preview.exists());
    }

    #[test]
    fn an_image_within_the_preview_size_gets_no_preview() {
        let history = history();
        history
            .record(png(800, 600), None, Bounds::default(), 1)
            .unwrap();
        let entry = history.entries().unwrap().remove(0);
        assert!(!preview_path(entry.path.unwrap()).exists());
        assert!(entry.thumbnail.is_some());
    }

    #[test]
    fn the_pane_shows_the_preview_of_a_large_image() {
        let history = history();
        history
            .record(png(1600, 1000), None, Bounds::default(), 1)
            .unwrap();
        let entry = history.entries().unwrap().remove(0);
        let pane = entry.pane_image().unwrap();
        assert!(pane.ends_with(".preview.png"));
        assert!(Path::new(&pane).exists());
    }

    #[test]
    fn the_pane_shows_an_image_within_the_preview_size_itself() {
        let history = history();
        history
            .record(png(800, 20), None, Bounds::default(), 1)
            .unwrap();
        let entry = history.entries().unwrap().remove(0);
        assert_eq!(entry.pane_image(), entry.path);
    }

    #[test]
    fn the_pane_shows_the_thumbnail_of_a_row_without_a_size() {
        let history = history();
        history
            .record(png(1600, 1000), None, Bounds::default(), 1)
            .unwrap();
        history
            .store
            .with(|c| {
                c.execute(
                    "UPDATE local_clipboard_history SET width = NULL, height = NULL",
                    [],
                )
            })
            .unwrap();
        let entry = history.entries().unwrap().remove(0);
        assert!(entry.thumbnail.is_some());
        assert_eq!(entry.pane_image(), entry.thumbnail);
    }

    #[test]
    fn a_text_entry_has_no_pane_image() {
        let history = history();
        history
            .record(text("a"), None, Bounds::default(), 1)
            .unwrap();
        assert_eq!(history.entries().unwrap()[0].pane_image(), None);
    }

    #[test]
    fn an_image_that_does_not_decode_is_recorded_without_a_thumbnail() {
        let history = history();
        history
            .record(image(64), None, Bounds::default(), 1)
            .unwrap();
        let entry = history.entries().unwrap().remove(0);
        assert!(entry.path.is_some());
        assert!(entry.thumbnail.is_none());
    }

    #[test]
    fn an_entry_whose_thumbnail_file_is_missing_reports_none() {
        let history = history();
        history
            .record(png(400, 10), None, Bounds::default(), 1)
            .unwrap();
        let thumbnail = history.entries().unwrap()[0].thumbnail.clone().unwrap();
        std::fs::remove_file(thumbnail).unwrap();

        assert!(history.entries().unwrap()[0].thumbnail.is_none());
    }

    #[test]
    fn copying_again_from_another_application_updates_source_and_time() {
        let history = history();
        history
            .record(text("one"), Some("Notepad"), Bounds::default(), 1)
            .unwrap();
        history
            .record(text("one"), Some("Code"), Bounds::default(), 2)
            .unwrap();
        let entry = history.entries().unwrap().remove(0);
        assert_eq!(entry.source.as_deref(), Some("Code"));
        assert_eq!(entry.copied_at, 2);
    }

    #[test]
    fn an_image_row_has_its_size() {
        let history = history();
        history
            .record(png(400, 30), None, Bounds::default(), 1)
            .unwrap();
        history
            .record(text("t"), None, Bounds::default(), 2)
            .unwrap();
        let entries = history.entries().unwrap();
        assert_eq!(entries[1].size, Some((400, 30)));
        assert_eq!(entries[0].size, None);
    }

    #[test]
    fn copying_the_same_thing_twice_reorders_instead_of_duplicating() {
        let history = history();
        history
            .record(text("one"), None, Bounds::default(), 1)
            .unwrap();
        history
            .record(text("two"), None, Bounds::default(), 2)
            .unwrap();
        history
            .record(text("one"), None, Bounds::default(), 3)
            .unwrap();

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
                .record(text(value), None, bounds, index as i64 + 1)
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
                .record(distinct_image(index, 100), None, bounds, index as i64 + 1)
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
        history
            .record(text("kept"), None, Bounds::default(), 1)
            .unwrap();
        let bounds = Bounds {
            entry_bytes: 10,
            ..Bounds::default()
        };

        let error = history.record(image(4096), None, bounds, 2).unwrap_err();
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
        history
            .record(text("worth keeping"), None, bounds, 1)
            .unwrap();

        let error = history
            .record(distinct_image(9, 4096), None, bounds, 2)
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
        history.record(png(400, 10), None, bounds, 1).unwrap();
        let entry = history.entries().unwrap().remove(0);
        let path = entry.path.unwrap();
        let thumbnail = entry.thumbnail.unwrap();

        history.record(png(400, 20), None, bounds, 2).unwrap();
        assert!(
            !Path::new(&path).exists(),
            "the discarded image must not stay on disk"
        );
        assert!(!Path::new(&thumbnail).exists());
    }

    #[test]
    fn removing_an_entry_deletes_its_file_too() {
        let history = history();
        history
            .record(png(400, 10), None, Bounds::default(), 1)
            .unwrap();
        let entry = history.entries().unwrap().remove(0);
        let path = entry.path.clone().unwrap();
        let thumbnail = entry.thumbnail.clone().unwrap();

        history.remove(&entry.id).unwrap();
        assert!(history.entries().unwrap().is_empty());
        assert!(!Path::new(&path).exists());
        assert!(!Path::new(&thumbnail).exists());
    }

    #[test]
    fn an_entry_whose_file_has_gone_is_reported_and_removed() {
        let history = history();
        history
            .record(image(32), None, Bounds::default(), 1)
            .unwrap();
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
        history
            .record(text("hello"), None, Bounds::default(), 1)
            .unwrap();
        history
            .record(image(16), None, Bounds::default(), 2)
            .unwrap();
        let entries = history.entries().unwrap();

        assert_eq!(history.content(&entries[1].id).unwrap(), text("hello"));
        assert_eq!(history.content(&entries[0].id).unwrap(), image(16));
    }

    #[test]
    fn an_image_without_a_row_is_deleted_on_start() {
        let store = Arc::new(Store::in_memory().unwrap());
        let dir = std::env::temp_dir().join(format!("dango-clip-{}", uuid::Uuid::new_v4()));
        let history = History::new(store.clone(), dir.clone());
        history
            .record(png(1000, 10), None, Bounds::default(), 1)
            .unwrap();
        let entry = history.entries().unwrap().remove(0);
        let orphan = dir.join("orphan.png");
        std::fs::write(&orphan, [1, 2, 3]).unwrap();
        let orphan_thumbnail = dir.join("orphan.thumb.png");
        std::fs::write(&orphan_thumbnail, [1, 2, 3]).unwrap();
        let orphan_preview = dir.join("orphan.preview.png");
        std::fs::write(&orphan_preview, [1, 2, 3]).unwrap();

        History::new(store, dir);

        assert!(!orphan.exists());
        assert!(!orphan_thumbnail.exists());
        assert!(!orphan_preview.exists());
        let path = entry.path.unwrap();
        assert!(Path::new(&path).exists());
        assert!(preview_path(&path).exists());
        assert!(Path::new(&entry.thumbnail.unwrap()).exists());
    }

    #[test]
    fn a_fingerprint_does_not_depend_on_the_toolchain() {
        assert_eq!(text("hello").fingerprint(), "ae8e2ff1480d4dd4");
    }

    #[test]
    fn the_list_holds_a_preview_and_the_content_is_whole() {
        let history = history();
        let long = "x".repeat(LISTED_CHARS + 10);
        history
            .record(text(&long), None, Bounds::default(), 1)
            .unwrap();
        let entry = history.entries().unwrap().remove(0);

        assert_eq!(entry.text.unwrap().len(), LISTED_CHARS);
        assert_eq!(history.content(&entry.id).unwrap(), text(&long));
    }

    #[test]
    fn the_listing_counts_lines_and_characters() {
        let history = history();
        history
            .record(text("one line"), None, Bounds::default(), 1)
            .unwrap();
        history
            .record(
                text("first\nsecond\nthird ünïcode"),
                None,
                Bounds::default(),
                2,
            )
            .unwrap();
        history
            .record(png(10, 10), None, Bounds::default(), 3)
            .unwrap();
        let entries = history.entries().unwrap();
        assert_eq!((entries[0].lines, entries[0].characters), (None, None));
        assert_eq!(
            (entries[1].lines, entries[1].characters),
            (Some(3), Some(26))
        );
        assert_eq!(
            (entries[2].lines, entries[2].characters),
            (Some(1), Some(8))
        );
    }

    #[test]
    fn a_long_text_counts_in_full() {
        let history = history();
        let long = "x".repeat(LISTED_CHARS + 10);
        history
            .record(text(&long), None, Bounds::default(), 1)
            .unwrap();
        let entry = history.entries().unwrap().remove(0);
        assert_eq!(entry.characters, Some(LISTED_CHARS as u32 + 10));
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

#[cfg(test)]
mod display_tests {
    use super::HistoryError;

    #[test]
    fn every_message_is_a_sentence() {
        let errors = [
            HistoryError::TooLarge,
            HistoryError::Gone,
            HistoryError::Failed("disk I/O error".into()),
        ];
        for error in errors {
            let text = error.to_string();
            assert!(text.starts_with(char::is_uppercase), "{text}");
            assert!(!text.ends_with('.') && !text.contains(". "), "{text}");
        }
    }
}
