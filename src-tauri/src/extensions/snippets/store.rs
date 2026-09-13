//! The user's snippets, and the same shape reused for quicklinks.
//!
//! Both are a name plus a template the user wrote, both are created and edited
//! from the launcher, and both should exist on either of the author's machines.
//! So one store serves both, and both live as text files in the config
//! directory rather than in the database, so they travel in the user's dotfiles.
//!
//! A file is an array of records, each `{ id, name, <body> }`, where the body
//! key is `template` for a snippet and `url` for a quicklink so the file reads
//! naturally. Records are held in memory and read from there on the search path;
//! the file is the durable form. Unknown fields on a record are preserved, so a
//! later feature adding, say, a keyword does not lose it.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

use serde_json::{Map, Value};

use crate::templates::{Template, TemplateError};

#[derive(Debug, thiserror::Error)]
pub enum RecordError {
    #[error("give it a name")]
    NoName,
    #[error("{0}")]
    NoBody(&'static str),
    #[error("{0}")]
    Template(#[from] TemplateError),
    #[error("that one is no longer there")]
    Gone,
    #[error("the records file is not valid JSON: {0}")]
    Parse(String),
    #[error("a snippet with a fill-in-the-blank cannot have a keyword")]
    KeywordNeedsPlainSnippet,
    #[error("another snippet already uses the keyword '{0}'")]
    KeywordTaken(String),
}

/// One snippet or quicklink as the store hands it back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub id: String,
    pub name: String,
    /// The template text: a snippet's body, or a quicklink's URL.
    pub body: String,
    /// The snippet's expansion keyword, if it has one. Always `None` for a
    /// quicklink.
    pub keyword: Option<String>,
}

/// Which of the two kinds a store instance owns. They are identical in shape,
/// differing only in the file they live in and what the body field is called.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Snippet,
    Quicklink,
}

impl Kind {
    pub fn file_name(self) -> &'static str {
        match self {
            Kind::Snippet => "snippets.json",
            Kind::Quicklink => "quicklinks.json",
        }
    }

    fn body_field(self) -> &'static str {
        match self {
            Kind::Snippet => "template",
            Kind::Quicklink => "url",
        }
    }

    fn missing_body(self) -> &'static str {
        match self {
            Kind::Snippet => "give it some text",
            Kind::Quicklink => "give it a URL",
        }
    }
}

pub struct Records {
    path: PathBuf,
    kind: Kind,
    /// The live set, each a JSON object so unknown fields survive.
    cache: RwLock<Vec<Map<String, Value>>>,
    /// The last text the app wrote, so the file watcher ignores its own write.
    own_write: Arc<Mutex<Option<String>>>,
}

impl Records {
    /// Opens the store for a kind under `config_dir`, loading its file. A missing
    /// file is an empty set. A malformed file is kept out of the cache and
    /// reported by `new` returning the parse error, so the caller can surface it
    /// while still running on an empty set.
    pub fn open(config_dir: &Path, kind: Kind) -> (Arc<Self>, Option<RecordError>) {
        let path = config_dir.join(kind.file_name());
        let (records, error) = match load(&path) {
            Ok(records) => (records, None),
            Err(error) => (Vec::new(), Some(error)),
        };
        let store = Arc::new(Self {
            path,
            kind,
            cache: RwLock::new(records),
            own_write: Arc::new(Mutex::new(None)),
        });
        (store, error)
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn own_write(&self) -> Arc<Mutex<Option<String>>> {
        self.own_write.clone()
    }

    /// Reloads the cache from `text`, for the file watcher. On a parse failure
    /// the last good cache is kept and the error returned to be surfaced.
    pub fn reload(&self, text: &str) -> Result<(), RecordError> {
        let records = parse(text)?;
        *self.cache.write().unwrap() = with_ids(records);
        Ok(())
    }

    fn validate(&self, name: &str, body: &str) -> Result<(), RecordError> {
        if name.trim().is_empty() {
            return Err(RecordError::NoName);
        }
        if body.trim().is_empty() {
            return Err(RecordError::NoBody(self.kind.missing_body()));
        }
        Template::parse(body)?;
        Ok(())
    }

    /// Checks a keyword before it is stored: a snippet whose template needs a
    /// fill-in-the-blank cannot have one, because expanding in place has no way
    /// to ask, and no two snippets may share a keyword. `own_id` is the record
    /// being edited, excluded from the uniqueness check. Returns the normalised
    /// keyword (trimmed, empty becomes `None`).
    fn validate_keyword(
        &self,
        keyword: Option<&str>,
        body: &str,
        own_id: Option<&str>,
    ) -> Result<Option<String>, RecordError> {
        let keyword = keyword.map(str::trim).filter(|k| !k.is_empty());
        let Some(keyword) = keyword else {
            return Ok(None);
        };
        if !Template::parse(body)?.arguments().is_empty() {
            return Err(RecordError::KeywordNeedsPlainSnippet);
        }
        let taken = self.cache.read().unwrap().iter().any(|record| {
            record_id(record) != own_id
                && record.get("keyword").and_then(Value::as_str) == Some(keyword)
        });
        if taken {
            return Err(RecordError::KeywordTaken(keyword.to_string()));
        }
        Ok(Some(keyword.to_string()))
    }

    pub fn create(
        &self,
        name: &str,
        body: &str,
        keyword: Option<&str>,
    ) -> Result<String, RecordError> {
        self.validate(name, body)?;
        let keyword = self.validate_keyword(keyword, body, None)?;
        let id = uuid::Uuid::new_v4().to_string();
        let mut record = Map::new();
        record.insert("id".into(), Value::String(id.clone()));
        record.insert("name".into(), Value::String(name.trim().to_string()));
        record.insert(
            self.kind.body_field().into(),
            Value::String(body.to_string()),
        );
        if let Some(keyword) = keyword {
            record.insert("keyword".into(), Value::String(keyword));
        }
        self.cache.write().unwrap().push(record);
        self.persist();
        Ok(id)
    }

    pub fn update(
        &self,
        id: &str,
        name: &str,
        body: &str,
        keyword: Option<&str>,
    ) -> Result<(), RecordError> {
        self.validate(name, body)?;
        let keyword = self.validate_keyword(keyword, body, Some(id))?;
        {
            let mut cache = self.cache.write().unwrap();
            let Some(record) = cache.iter_mut().find(|r| record_id(r) == Some(id)) else {
                return Err(RecordError::Gone);
            };
            record.insert("name".into(), Value::String(name.trim().to_string()));
            record.insert(
                self.kind.body_field().into(),
                Value::String(body.to_string()),
            );
            match keyword {
                Some(keyword) => {
                    record.insert("keyword".into(), Value::String(keyword));
                }
                None => {
                    record.remove("keyword");
                }
            }
        }
        self.persist();
        Ok(())
    }

    /// Removes the record's entry outright. There is no tombstone: git remembers
    /// what was deleted, so a hard delete does not come back.
    pub fn remove(&self, id: &str) -> Result<(), RecordError> {
        {
            let mut cache = self.cache.write().unwrap();
            let before = cache.len();
            cache.retain(|r| record_id(r) != Some(id));
            if cache.len() == before {
                return Err(RecordError::Gone);
            }
        }
        self.persist();
        Ok(())
    }

    pub fn all(&self) -> Result<Vec<Record>, RecordError> {
        let cache = self.cache.read().unwrap();
        let mut records: Vec<Record> = cache
            .iter()
            .filter_map(|record| self.project(record))
            .collect();
        records.sort_by_key(|record| record.name.to_lowercase());
        Ok(records)
    }

    pub fn get(&self, id: &str) -> Result<Record, RecordError> {
        let cache = self.cache.read().unwrap();
        cache
            .iter()
            .find(|r| record_id(r) == Some(id))
            .and_then(|record| self.project(record))
            .ok_or(RecordError::Gone)
    }

    /// A stored object as a `Record`, or `None` if it has no name to show.
    fn project(&self, record: &Map<String, Value>) -> Option<Record> {
        let name = record.get("name")?.as_str()?.to_string();
        let body = record
            .get(self.kind.body_field())
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let id = record_id(record)?.to_string();
        let keyword = match self.kind {
            Kind::Snippet => record
                .get("keyword")
                .and_then(Value::as_str)
                .map(str::to_string),
            Kind::Quicklink => None,
        };
        Some(Record {
            id,
            name,
            body,
            keyword,
        })
    }

    fn persist(&self) {
        let text = {
            let cache = self.cache.read().unwrap();
            serde_json::to_string_pretty(&*cache).unwrap_or_else(|_| "[]".to_string())
        };
        *self.own_write.lock().unwrap() = Some(text.clone());
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(error) = std::fs::write(&self.path, text) {
            eprintln!("[dango] could not write {}: {error}", self.path.display());
        }
    }
}

fn record_id(record: &Map<String, Value>) -> Option<&str> {
    record.get("id").and_then(|v| v.as_str())
}

/// Loads and parses a records file. A missing file is an empty set. Ids are
/// filled in for any entry that lacks one.
fn load(path: &Path) -> Result<Vec<Map<String, Value>>, RecordError> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(with_ids(parse(&text)?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(RecordError::Parse(error.to_string())),
    }
}

fn parse(text: &str) -> Result<Vec<Map<String, Value>>, RecordError> {
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(text).map_err(|error| RecordError::Parse(error.to_string()))
}

/// Assigns an id to any record that was hand-authored without one, so it has a
/// stable identity from now on. The id is written the next time the file is.
fn with_ids(mut records: Vec<Map<String, Value>>) -> Vec<Map<String, Value>> {
    for record in &mut records {
        if record_id(record).is_none() {
            record.insert("id".into(), Value::String(uuid::Uuid::new_v4().to_string()));
        }
    }
    records
}

#[cfg(test)]
mod tests {
    use super::*;

    fn records(kind: Kind) -> Arc<Records> {
        let dir = std::env::temp_dir().join(format!("dango-records-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        Records::open(&dir, kind).0
    }

    #[test]
    fn a_snippet_is_stored_and_read_back() {
        let records = records(Kind::Snippet);
        let id = records.create("Signature", "Best, Nico", None).unwrap();

        let all = records.all().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, id);
        assert_eq!(all[0].name, "Signature");
        assert_eq!(all[0].body, "Best, Nico");
    }

    #[test]
    fn a_snippet_survives_reopening_the_store() {
        let dir = std::env::temp_dir().join(format!("dango-records-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let id = {
            let records = Records::open(&dir, Kind::Snippet).0;
            records.create("Signature", "Best, Nico", None).unwrap()
        };
        let reopened = Records::open(&dir, Kind::Snippet).0;
        let all = reopened.all().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, id, "the id is stable across reopen");
    }

    #[test]
    fn a_snippet_is_edited_and_the_old_version_is_not_kept() {
        let records = records(Kind::Snippet);
        let id = records.create("Signature", "Best, Nico", None).unwrap();

        records
            .update(&id, "Sign-off", "Regards, Nico", None)
            .unwrap();

        let all = records.all().unwrap();
        assert_eq!(all.len(), 1, "an edit is not a second snippet");
        assert_eq!(all[0].name, "Sign-off");
        assert_eq!(all[0].body, "Regards, Nico");
    }

    #[test]
    fn a_removed_snippet_is_gone_and_leaves_no_entry() {
        let records = records(Kind::Snippet);
        let id = records.create("Signature", "Best, Nico", None).unwrap();

        records.remove(&id).unwrap();

        assert!(records.all().unwrap().is_empty());
        assert!(matches!(records.get(&id), Err(RecordError::Gone)));
        // Hard delete: nothing left in the file, no tombstone.
        let on_disk = std::fs::read_to_string(records.path()).unwrap();
        assert!(!on_disk.contains(&id));
    }

    #[test]
    fn create_update_and_remove_write_the_file() {
        let records = records(Kind::Snippet);
        let id = records.create("Signature", "Best, Nico", None).unwrap();
        assert!(std::fs::read_to_string(records.path())
            .unwrap()
            .contains("Signature"));
        records.update(&id, "Sign-off", "Regards", None).unwrap();
        assert!(std::fs::read_to_string(records.path())
            .unwrap()
            .contains("Sign-off"));
        records.remove(&id).unwrap();
        assert_eq!(records.all().unwrap().len(), 0);
    }

    #[test]
    fn a_hand_added_entry_gains_an_id_and_is_preserved() {
        let dir = std::env::temp_dir().join(format!("dango-records-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("snippets.json"),
            r#"[{ "name": "Hand", "template": "typed by hand" }]"#,
        )
        .unwrap();
        let records = Records::open(&dir, Kind::Snippet).0;
        let all = records.all().unwrap();
        assert_eq!(all.len(), 1);
        assert!(!all[0].id.is_empty(), "an id was assigned");
        assert_eq!(all[0].body, "typed by hand");
    }

    #[test]
    fn unknown_fields_on_a_record_are_preserved() {
        let dir = std::env::temp_dir().join(format!("dango-records-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("snippets.json"),
            r#"[{ "id": "a", "name": "Hand", "template": "t", "color": "blue" }]"#,
        )
        .unwrap();
        let records = Records::open(&dir, Kind::Snippet).0;
        // An edit that does not touch the unknown field keeps it.
        records.update("a", "Hand", "t2", None).unwrap();
        let on_disk = std::fs::read_to_string(records.path()).unwrap();
        assert!(
            on_disk.contains("color"),
            "unknown field dropped: {on_disk}"
        );
        assert!(on_disk.contains("blue"));
    }

    #[test]
    fn a_malformed_file_reports_and_keeps_the_last_good_records() {
        let dir = std::env::temp_dir().join(format!("dango-records-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let records = Records::open(&dir, Kind::Snippet).0;
        records.create("Signature", "Best", None).unwrap();
        // A bad edit lands; reload keeps the last good set.
        let error = records.reload("{ not an array").unwrap_err();
        assert!(matches!(error, RecordError::Parse(_)));
        assert_eq!(records.all().unwrap().len(), 1, "last good kept");
    }

    #[test]
    fn a_snippet_needs_a_name() {
        let records = records(Kind::Snippet);
        assert!(matches!(
            records.create("   ", "Best, Nico", None),
            Err(RecordError::NoName)
        ));
        assert!(records.all().unwrap().is_empty());
    }

    #[test]
    fn a_snippet_needs_some_text() {
        let records = records(Kind::Snippet);
        assert!(matches!(
            records.create("Signature", "", None),
            Err(RecordError::NoBody(_))
        ));
        assert!(records.all().unwrap().is_empty());
    }

    #[test]
    fn a_template_that_will_not_parse_is_refused_where_it_is_written() {
        let records = records(Kind::Snippet);
        let error = records
            .create("Broken", "unclosed {{ name", None)
            .unwrap_err();
        assert!(matches!(error, RecordError::Template(_)));
        assert!(records.all().unwrap().is_empty());
    }

    #[test]
    fn editing_something_that_is_gone_says_so() {
        let records = records(Kind::Snippet);
        assert!(matches!(
            records.update("nope", "Name", "Body", None),
            Err(RecordError::Gone)
        ));
    }

    #[test]
    fn quicklinks_use_their_own_file_and_body_key() {
        let dir = std::env::temp_dir().join(format!("dango-records-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let snippets = Records::open(&dir, Kind::Snippet).0;
        let quicklinks = Records::open(&dir, Kind::Quicklink).0;

        snippets.create("Signature", "Best, Nico", None).unwrap();
        quicklinks
            .create("Search", "https://example.com?q={{ query }}", None)
            .unwrap();

        assert_eq!(snippets.all().unwrap().len(), 1);
        assert_eq!(quicklinks.all().unwrap().len(), 1);
        // The quicklink file uses "url" as its body key.
        let on_disk = std::fs::read_to_string(quicklinks.path()).unwrap();
        assert!(on_disk.contains("\"url\""), "quicklink body key: {on_disk}");
    }

    #[test]
    fn a_quicklink_says_it_needs_a_url() {
        let records = records(Kind::Quicklink);
        let error = records.create("Search", "  ", None).unwrap_err();
        assert_eq!(error.to_string(), "give it a URL");
    }

    #[test]
    fn records_come_back_in_name_order_regardless_of_case() {
        let records = records(Kind::Snippet);
        records.create("zebra", "z", None).unwrap();
        records.create("Apple", "a", None).unwrap();
        records.create("mango", "m", None).unwrap();

        let names: Vec<_> = records
            .all()
            .unwrap()
            .into_iter()
            .map(|record| record.name)
            .collect();
        assert_eq!(names, vec!["Apple", "mango", "zebra"]);
    }

    #[test]
    fn a_snippet_stores_and_reads_its_keyword() {
        let records = records(Kind::Snippet);
        let id = records
            .create("Signature", "Best, Nico", Some(";sig"))
            .unwrap();
        assert_eq!(records.get(&id).unwrap().keyword.as_deref(), Some(";sig"));
        // An absent keyword is None, and an empty one is treated as absent.
        let plain = records.create("Plain", "hi", Some("  ")).unwrap();
        assert_eq!(records.get(&plain).unwrap().keyword, None);
    }

    #[test]
    fn a_keyword_can_be_cleared_on_edit() {
        let records = records(Kind::Snippet);
        let id = records
            .create("Signature", "Best, Nico", Some(";sig"))
            .unwrap();
        records
            .update(&id, "Signature", "Best, Nico", None)
            .unwrap();
        assert_eq!(records.get(&id).unwrap().keyword, None);
    }

    #[test]
    fn a_keyword_on_a_fill_in_the_blank_is_refused() {
        let records = records(Kind::Snippet);
        let error = records
            .create("Greeting", "Dear {{ name }}", Some(";hi"))
            .unwrap_err();
        assert!(matches!(error, RecordError::KeywordNeedsPlainSnippet));
        // A self-resolving placeholder is not a fill-in-the-blank, so it is fine.
        assert!(records.create("Today", "{{ date }}", Some(";d")).is_ok());
    }

    #[test]
    fn a_duplicate_keyword_is_refused() {
        let records = records(Kind::Snippet);
        records.create("One", "first", Some(";sig")).unwrap();
        let error = records.create("Two", "second", Some(";sig")).unwrap_err();
        assert!(matches!(error, RecordError::KeywordTaken(k) if k == ";sig"));
        // Re-saving the same record keeps its own keyword.
        let id = records.create("Three", "third", Some(";x")).unwrap();
        assert!(records.update(&id, "Three", "third", Some(";x")).is_ok());
    }
}
