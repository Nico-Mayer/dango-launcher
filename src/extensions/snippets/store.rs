//! The user's snippets, and the same shape reused for quicklinks.
//!
//! Both are a name plus a template the user wrote, both are created and edited
//! from the launcher, and both should exist on either of the author's machines.
//! So one store serves both, and both live as TOML files in the config
//! directory rather than in the database, so they travel in the user's dotfiles.
//!
//! A file holds one table per record, keyed by its id, with `name` and a body
//! key: `template` for a snippet, `url` for a quicklink. The store keeps the
//! parsed document and edits only the table it changes, so the user's comments,
//! order, and unknown keys survive Dango's writes. Records are read from memory
//! on the search path; the file is the durable form.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use schemars::JsonSchema;
use toml_edit::{DocumentMut, Item, Table, TableLike};

use crate::browser::Browser;
use crate::config::{line_of, write_user_file, UserFile};
use crate::templates::{Template, TemplateError, Values};

use super::favicons::site_of;
use super::matcher::KeywordEntry;

/// Reserved at the top level of a records file, so never a record id.
const VERSION: &str = "version";

#[derive(Debug, thiserror::Error)]
pub enum RecordError {
    #[error("Enter a name")]
    NoName,
    #[error("{0}")]
    NoBody(&'static str),
    #[error("{0}")]
    Template(#[from] TemplateError),
    #[error("Enter a URL such as https://example.com")]
    InvalidUrl,
    #[error("That no longer exists")]
    Gone,
    #[error("{0}")]
    Parse(String),
    #[error("Line {line}: {field} must be text")]
    NotText { line: usize, field: String },
    #[error("Couldn't read the file: {0}")]
    Read(String),
    #[error("A snippet that asks for input can't have a keyword")]
    KeywordNeedsPlainSnippet,
    #[error("Another snippet already uses that keyword")]
    KeywordTaken(String),
    #[error("One keyword can't be the start of another, so choose a different keyword")]
    KeywordOverlaps(String),
    #[error("Fix the error in {0} first")]
    FileBroken(&'static str),
    #[error("Couldn't save, check that {0} isn't read-only")]
    Write(&'static str),
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
    /// The browser id a quicklink opens in, as stored. Always `None` for a
    /// snippet.
    pub browser: Option<String>,
}

/// A snippet, keyed by its id.
#[derive(JsonSchema)]
pub struct SnippetEntry {
    /// The title shown in the launcher.
    pub name: String,
    /// The text to insert. Placeholders such as `{{ date }}` are filled in
    /// when the snippet is used.
    pub template: String,
    /// Typing this anywhere expands the snippet in place. Not allowed on a
    /// snippet that asks for input.
    pub keyword: Option<String>,
}

/// A quicklink, keyed by its id.
#[derive(JsonSchema)]
pub struct QuicklinkEntry {
    /// The title shown in the launcher.
    pub name: String,
    /// The URL to open. `{{ query }}` asks for text and places it in the URL.
    pub url: String,
    /// The browser to open it in. Left out, the default browser opens it.
    pub browser: Option<String>,
}

/// Which of the two kinds a store instance owns. They are identical in shape,
/// differing only in the file they live in and what the body field is called.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Snippet,
    Quicklink,
}

impl Kind {
    pub fn file(self) -> UserFile {
        match self {
            Kind::Snippet => UserFile::Snippets,
            Kind::Quicklink => UserFile::Quicklinks,
        }
    }

    pub fn file_name(self) -> &'static str {
        self.file().file_name()
    }

    fn body_field(self) -> &'static str {
        match self {
            Kind::Snippet => "template",
            Kind::Quicklink => "url",
        }
    }

    fn missing_body(self) -> &'static str {
        match self {
            Kind::Snippet => "Enter the snippet text",
            Kind::Quicklink => "Enter a URL",
        }
    }

    /// The id for a name that has no letters or digits to make one from.
    fn fallback_id(self) -> &'static str {
        match self {
            Kind::Snippet => "snippet",
            Kind::Quicklink => "quicklink",
        }
    }
}

/// The file as parsed: the document Dango edits, and its records.
#[derive(Clone)]
struct Loaded {
    doc: DocumentMut,
    records: Vec<Record>,
}

impl Loaded {
    fn empty() -> Self {
        Self {
            doc: DocumentMut::new(),
            records: Vec::new(),
        }
    }
}

pub struct Records {
    path: PathBuf,
    kind: Kind,
    cache: RwLock<Loaded>,
    /// The last text the app wrote, so the file watcher ignores its own write.
    own_write: Arc<Mutex<Option<String>>>,
    /// The file failed to load, so the cache is not what the file holds and a
    /// write would replace the user's records with it.
    broken: AtomicBool,
    /// Each quicklink's host by id, worked out when the set changes so root
    /// search does not parse every template on each keystroke.
    hosts: RwLock<HashMap<String, String>>,
}

impl Records {
    /// Opens the store for a kind under `config_dir`, loading its file. A missing
    /// file is an empty set. A malformed file is kept out of the cache and
    /// reported by returning the error, so the caller can surface it while still
    /// running on an empty set.
    pub fn open(config_dir: &Path, kind: Kind) -> (Arc<Self>, Option<RecordError>) {
        let path = kind.file().path_in(config_dir);
        let (loaded, error) = match load(&path, kind) {
            Ok(loaded) => (loaded, None),
            Err(error) => (Loaded::empty(), Some(error)),
        };
        let store = Arc::new(Self {
            path,
            kind,
            broken: AtomicBool::new(error.is_some()),
            cache: RwLock::new(Loaded::empty()),
            own_write: Arc::new(Mutex::new(None)),
            hosts: RwLock::new(HashMap::new()),
        });
        store.set_hosts(&loaded.records);
        *store.cache.write().unwrap() = loaded;
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
        let loaded =
            parse(text, self.kind).inspect_err(|_| self.broken.store(true, Ordering::SeqCst))?;
        self.set_hosts(&loaded.records);
        *self.cache.write().unwrap() = loaded;
        self.broken.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn set_hosts(&self, records: &[Record]) {
        if self.kind != Kind::Quicklink {
            return;
        }
        let hosts = records
            .iter()
            .filter_map(|record| {
                let (host, _) = site_of(record)?;
                Some((record.id.clone(), host))
            })
            .collect();
        *self.hosts.write().unwrap() = hosts;
    }

    /// The host a quicklink opens, if its template resolves to a web URL.
    pub fn host(&self, id: &str) -> Option<String> {
        self.hosts.read().unwrap().get(id).cloned()
    }

    fn writable(&self) -> Result<(), RecordError> {
        if self.broken.load(Ordering::SeqCst) {
            return Err(RecordError::FileBroken(self.kind.file_name()));
        }
        Ok(())
    }

    fn validate(&self, name: &str, body: &str) -> Result<(), RecordError> {
        if name.trim().is_empty() {
            return Err(RecordError::NoName);
        }
        if body.trim().is_empty() {
            return Err(RecordError::NoBody(self.kind.missing_body()));
        }
        let template = Template::parse(body)?;
        if self.kind == Kind::Quicklink {
            let rendered = template.render_url(&Values::default())?;
            url::Url::parse(&rendered.text).map_err(|_| RecordError::InvalidUrl)?;
        }
        Ok(())
    }

    /// Checks a keyword before it is stored: a snippet whose template needs a
    /// fill-in-the-blank cannot have one, because expanding in place has no way
    /// to ask, and no two snippets may share a keyword or have one start
    /// with the other. `own_id` is the record
    /// being edited, excluded from these checks. Returns the normalised
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
        let cache = self.cache.read().unwrap();
        let others: Vec<&str> = cache
            .records
            .iter()
            .filter(|record| Some(record.id.as_str()) != own_id)
            .filter_map(|record| record.keyword.as_deref())
            .collect();
        if others.contains(&keyword) {
            return Err(RecordError::KeywordTaken(keyword.to_string()));
        }
        if let Some(other) = others
            .iter()
            .find(|other| keyword.starts_with(**other) || other.starts_with(keyword))
        {
            return Err(RecordError::KeywordOverlaps(other.to_string()));
        }
        Ok(Some(keyword.to_string()))
    }

    #[cfg(test)]
    pub fn create(
        &self,
        name: &str,
        body: &str,
        keyword: Option<&str>,
    ) -> Result<String, RecordError> {
        self.save(None, name, body, keyword, None)
    }

    #[cfg(test)]
    pub fn update(
        &self,
        id: &str,
        name: &str,
        body: &str,
        keyword: Option<&str>,
    ) -> Result<(), RecordError> {
        self.save(Some(id), name, body, keyword, None).map(|_| ())
    }

    /// Creates the record when `id` is `None`, else updates it, and returns its
    /// id. `browser` is the form's value: `None` leaves the field as it is,
    /// `default` or empty removes it, and anything else is stored as given, so a
    /// browser this platform does not offer survives an edit.
    pub fn save(
        &self,
        id: Option<&str>,
        name: &str,
        body: &str,
        keyword: Option<&str>,
        browser: Option<&str>,
    ) -> Result<String, RecordError> {
        self.writable()?;
        self.validate(name, body)?;
        let keyword = self.validate_keyword(keyword, body, id)?;
        let kind = self.kind;
        let mut saved = id.map(str::to_string);
        self.change(|doc| {
            let id = match &saved {
                Some(id) => id.clone(),
                None => {
                    let id = new_id(doc, name, kind);
                    doc.insert(&id, Item::Table(Table::new()));
                    saved = Some(id.clone());
                    id
                }
            };
            let table = record_table(doc, &id).ok_or(RecordError::Gone)?;
            set_text(table, "name", name.trim());
            set_text(table, kind.body_field(), body);
            if kind == Kind::Snippet {
                set_or_remove(table, "keyword", keyword);
            }
            match browser.map(str::trim) {
                None => {}
                Some("") | Some(Browser::DEFAULT_ID) => set_or_remove(table, "browser", None),
                Some(browser) => set_or_remove(table, "browser", Some(browser.to_string())),
            }
            Ok(())
        })?;
        Ok(saved.unwrap_or_default())
    }

    /// Removes the record's table outright. There is no tombstone: git remembers
    /// what was deleted, so a hard delete does not come back.
    pub fn remove(&self, id: &str) -> Result<(), RecordError> {
        self.writable()?;
        self.change(|doc| {
            record_table(doc, id).ok_or(RecordError::Gone)?;
            doc.remove(id);
            Ok(())
        })
    }

    /// Applies `edit` to a copy of the document and keeps it only once the file
    /// holds it, so a failed write leaves the cache as the file has it.
    fn change(
        &self,
        edit: impl FnOnce(&mut DocumentMut) -> Result<(), RecordError>,
    ) -> Result<(), RecordError> {
        let mut cache = self.cache.write().unwrap();
        let mut doc = cache.doc.clone();
        edit(&mut doc)?;
        let mut text = doc.to_string();
        if !self.path.exists() {
            text = format!("{}\n{text}", self.kind.file().header());
        }
        let next = parse(&text, self.kind)?;
        if let Err(error) = write_user_file(&self.path, &text, &self.own_write) {
            crate::log::append(&format!("could not write {}: {error}", self.path.display()));
            return Err(RecordError::Write(self.kind.file_name()));
        }
        self.set_hosts(&next.records);
        *cache = next;
        Ok(())
    }

    /// The snippets' keywords, read without copying or sorting the records,
    /// because the expansion worker asks on every keystroke.
    pub fn keywords(&self) -> Vec<KeywordEntry> {
        self.cache
            .read()
            .unwrap()
            .records
            .iter()
            .filter_map(|record| {
                Some(KeywordEntry {
                    id: record.id.clone(),
                    keyword: record.keyword.clone()?,
                })
            })
            .collect()
    }

    pub fn all(&self) -> Result<Vec<Record>, RecordError> {
        let mut records = self.cache.read().unwrap().records.clone();
        records.sort_by_key(|record| record.name.to_lowercase());
        Ok(records)
    }

    /// The records as the file has them now, for a caller that follows the file
    /// itself and must not read a set the watcher has not reloaded yet.
    pub fn on_disk(&self) -> Result<Vec<Record>, RecordError> {
        Ok(load(&self.path, self.kind)?.records)
    }

    pub fn get(&self, id: &str) -> Result<Record, RecordError> {
        self.cache
            .read()
            .unwrap()
            .records
            .iter()
            .find(|record| record.id == id)
            .cloned()
            .ok_or(RecordError::Gone)
    }
}

/// An id made from the name, numbered when taken. Never `version`, which the
/// file reserves.
fn new_id(doc: &DocumentMut, name: &str, kind: Kind) -> String {
    let base = Some(slug::slugify(name))
        .filter(|slug| !slug.is_empty())
        .unwrap_or_else(|| kind.fallback_id().to_string());
    let mut id = base.clone();
    let mut count = 1;
    while id == VERSION || doc.contains_key(&id) {
        count += 1;
        id = format!("{base}-{count}");
    }
    id
}

fn record_table<'d>(doc: &'d mut DocumentMut, id: &str) -> Option<&'d mut dyn TableLike> {
    if id == VERSION {
        return None;
    }
    doc.get_mut(id)?.as_table_like_mut()
}

/// Sets a field only when its text changes, and keeps the comment beside it:
/// replacing a value in `toml_edit` replaces its decor too.
fn set_text(table: &mut dyn TableLike, key: &str, text: &str) {
    let old = table.get(key).and_then(Item::as_value);
    if old.and_then(|old| old.as_str()) == Some(text) {
        return;
    }
    let mut new = toml_edit::Value::from(text);
    if let Some(old) = old {
        *new.decor_mut() = old.decor().clone();
    }
    table.insert(key, Item::Value(new));
}

fn set_or_remove(table: &mut dyn TableLike, key: &str, text: Option<String>) {
    match text {
        Some(text) => set_text(table, key, &text),
        None => {
            table.remove(key);
        }
    }
}

/// Loads and parses a records file. A missing file is an empty set.
fn load(path: &Path, kind: Kind) -> Result<Loaded, RecordError> {
    match std::fs::read_to_string(path) {
        Ok(text) => parse(&text, kind),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Loaded::empty()),
        Err(error) => Err(RecordError::Read(error.to_string())),
    }
}

/// Parses a records file. A table without a name is not shown, and a field
/// of the wrong type makes the file invalid with its line, as a wrong type
/// does in the config file. A top-level key that is not a table is not a
/// record, so it is left for whatever understands it.
fn parse(text: &str, kind: Kind) -> Result<Loaded, RecordError> {
    let document =
        toml_edit::Document::parse(text).map_err(|error| RecordError::Parse(error.to_string()))?;
    let mut records = Vec::new();
    for (id, item) in document.as_table().iter() {
        if id == VERSION {
            continue;
        }
        let Some(table) = item.as_table_like() else {
            continue;
        };
        if let Some(record) = project(text, kind, id, table)? {
            records.push(record);
        }
    }
    Ok(Loaded {
        doc: document.into_mut(),
        records,
    })
}

fn project(
    text: &str,
    kind: Kind,
    id: &str,
    table: &dyn TableLike,
) -> Result<Option<Record>, RecordError> {
    let field = |field: &str| -> Result<Option<String>, RecordError> {
        let Some(item) = table.get(field) else {
            return Ok(None);
        };
        match item.as_str() {
            Some(text) => Ok(Some(text.to_string())),
            None => Err(RecordError::NotText {
                line: line_of(text, item.span()),
                field: field.to_string(),
            }),
        }
    };
    let name = field("name")?;
    let body = field(kind.body_field())?.unwrap_or_default();
    let keyword = match kind {
        Kind::Snippet => field("keyword")?,
        Kind::Quicklink => None,
    };
    let browser = match kind {
        Kind::Snippet => None,
        Kind::Quicklink => field("browser")?,
    };
    Ok(name.map(|name| Record {
        id: id.to_string(),
        name,
        body,
        keyword,
        browser,
    }))
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
        assert!(!records.path().with_extension("toml.tmp").exists());
    }

    fn dir_with(file: &str, text: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dango-records-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(file), text).unwrap();
        dir
    }

    #[test]
    fn a_hand_added_record_is_read_under_its_key() {
        let dir = dir_with(
            "snippets.toml",
            "[brb]\nname = \"Be right back\"\ntemplate = \"be right back\"\n",
        );
        let records = Records::open(&dir, Kind::Snippet).0;
        let all = records.all().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "brb");
        assert_eq!(all[0].body, "be right back");
    }

    #[test]
    fn an_inline_table_is_a_record_too() {
        let records = records(Kind::Quicklink);
        records
            .reload("github = { name = \"GitHub\", url = \"https://github.com\" }\n")
            .unwrap();
        records
            .update("github", "GitHub", "https://github.com/notifications", None)
            .unwrap();
        assert_eq!(
            records.get("github").unwrap().body,
            "https://github.com/notifications"
        );
    }

    #[test]
    fn version_is_not_a_record() {
        let records = records(Kind::Snippet);
        records
            .reload("version = 1\n\n[a]\nname = \"A\"\ntemplate = \"a\"\n")
            .unwrap();
        let ids: Vec<_> = records.all().unwrap().into_iter().map(|r| r.id).collect();
        assert_eq!(ids, ["a"]);
        assert!(matches!(records.remove(VERSION), Err(RecordError::Gone)));
        let id = records.create("Version", "v", None).unwrap();
        assert_eq!(id, "version-2");
    }

    #[test]
    fn ids_are_made_from_the_name() {
        let records = records(Kind::Snippet);
        assert_eq!(records.create("Café menu", "x", None).unwrap(), "cafe-menu");
        assert_eq!(
            records.create("Café Menu", "y", None).unwrap(),
            "cafe-menu-2"
        );
        assert_eq!(records.create("!!!", "z", None).unwrap(), "snippet");
        assert_eq!(records.create("???", "w", None).unwrap(), "snippet-2");
        let quicklinks = self::records(Kind::Quicklink);
        assert_eq!(
            quicklinks.create("…", "https://example.com", None).unwrap(),
            "quicklink"
        );
    }

    #[test]
    fn a_rename_keeps_the_id() {
        let records = records(Kind::Snippet);
        let id = records.create("Signature", "Best", None).unwrap();
        records.update(&id, "Work signature", "Best", None).unwrap();
        assert_eq!(id, "signature");
        assert_eq!(records.get("signature").unwrap().name, "Work signature");
        let on_disk = std::fs::read_to_string(records.path()).unwrap();
        assert!(on_disk.contains("[signature]"), "{on_disk}");
    }

    #[test]
    fn a_created_file_points_at_its_schema() {
        let records = records(Kind::Snippet);
        records.create("Signature", "Best", None).unwrap();
        let on_disk = std::fs::read_to_string(records.path()).unwrap();
        assert!(
            on_disk.starts_with("#:schema ./.schemas/snippets.schema.json\n"),
            "{on_disk}"
        );
        records.create("Second", "Two", None).unwrap();
        let on_disk = std::fs::read_to_string(records.path()).unwrap();
        assert_eq!(on_disk.matches("#:schema").count(), 1, "{on_disk}");
    }

    #[test]
    fn a_multi_line_template_is_written_on_several_lines() {
        let records = records(Kind::Snippet);
        records.create("Signature", "Best,\nNico", None).unwrap();
        let on_disk = std::fs::read_to_string(records.path()).unwrap();
        assert!(
            on_disk.contains("\"\"\"") && on_disk.contains("Best,\nNico"),
            "{on_disk}"
        );
        assert_eq!(records.get("signature").unwrap().body, "Best,\nNico");
    }

    #[test]
    fn comments_order_and_unknown_keys_survive_writes() {
        let text = "#:schema ./.schemas/quicklinks.schema.json\n\n\
            # Work\n[one]\nname = \"One\"\nurl = \"https://one.example\"\ncolor = \"blue\"\n\n\
            [two]\nname = \"Two\" # the second\nurl = \"https://two.example\"\n\n\
            [three]\nname = \"Three\"\nurl = \"https://three.example\"\n";
        let dir = dir_with("quicklinks.toml", text);
        let records = Records::open(&dir, Kind::Quicklink).0;

        records
            .update("two", "Two", "https://two.example/new", None)
            .unwrap();
        records
            .create("Four", "https://four.example", None)
            .unwrap();

        let on_disk = std::fs::read_to_string(records.path()).unwrap();
        assert!(on_disk.starts_with(
            "#:schema ./.schemas/quicklinks.schema.json\n\n# Work\n[one]\nname = \"One\"\nurl = \"https://one.example\"\ncolor = \"blue\"\n"
        ), "{on_disk}");
        let order: Vec<usize> = ["[one]", "[two]", "[three]", "[four]"]
            .iter()
            .map(|header| on_disk.find(header).unwrap())
            .collect();
        assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{on_disk}");
        assert!(on_disk.contains("https://two.example/new"), "{on_disk}");
        assert!(on_disk.contains("name = \"Two\" # the second"), "{on_disk}");
    }

    #[test]
    fn a_field_of_the_wrong_type_names_its_line() {
        let records = records(Kind::Snippet);
        records.create("Signature", "Best", None).unwrap();
        let error = records
            .reload("[a]\nname = \"A\"\ntemplate = \"a\"\n\n[b]\nname = 3\n")
            .unwrap_err();
        assert_eq!(error.to_string(), "Line 6: name must be text");
        assert_eq!(records.all().unwrap().len(), 1, "last good kept");
        let error = records.reload("[a]\ntemplate = 3\n").unwrap_err();
        assert_eq!(error.to_string(), "Line 2: template must be text");
    }

    #[test]
    fn an_unknown_top_level_key_is_kept_and_ignored() {
        let records = records(Kind::Quicklink);
        records
            .reload("default-browser = \"firefox\"\n\n[docs]\nname = \"Docs\"\nurl = \"https://example.com\"\n")
            .unwrap();
        assert_eq!(records.all().unwrap().len(), 1);
        records
            .update("docs", "Docs", "https://example.com/new", None)
            .unwrap();
        let on_disk = std::fs::read_to_string(records.path()).unwrap();
        assert!(
            on_disk.contains("default-browser = \"firefox\""),
            "{on_disk}"
        );
    }

    #[test]
    fn a_malformed_file_reports_and_keeps_the_last_good_records() {
        let dir = std::env::temp_dir().join(format!("dango-records-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let records = Records::open(&dir, Kind::Snippet).0;
        records.create("Signature", "Best", None).unwrap();
        // A bad edit lands; reload keeps the last good set.
        let error = records.reload("[a\nname = 1").unwrap_err();
        assert!(matches!(error, RecordError::Parse(_)));
        assert!(error.to_string().contains("line 1"), "{error}");
        assert_eq!(records.all().unwrap().len(), 1, "last good kept");
    }

    #[test]
    fn a_file_that_failed_to_load_is_not_overwritten() {
        let broken = "[hand]\nname = \"Hand\"\ntemplate = \"t\n";
        let dir = dir_with("snippets.toml", broken);
        let (records, error) = Records::open(&dir, Kind::Snippet);
        assert!(error.is_some());

        let refused = records.create("Signature", "Best", None).unwrap_err();
        assert!(matches!(refused, RecordError::FileBroken("snippets.toml")));
        assert_eq!(std::fs::read_to_string(records.path()).unwrap(), broken);

        records
            .reload("[hand]\nname = \"Hand\"\ntemplate = \"t\"\n")
            .unwrap();
        assert!(records.create("Signature", "Best", None).is_ok());
    }

    #[test]
    fn a_failed_write_is_reported_and_not_kept() {
        let dir = std::env::temp_dir().join(format!("dango-records-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let records = Records::open(&dir, Kind::Snippet).0;
        // A directory where the file should be makes every write fail.
        std::fs::create_dir(records.path()).unwrap();

        let error = records.create("Signature", "Best", None).unwrap_err();
        assert!(matches!(error, RecordError::Write("snippets.toml")));
        assert!(records.all().unwrap().is_empty());
    }

    #[test]
    fn a_bad_reload_refuses_writes_until_a_clean_one() {
        let records = records(Kind::Snippet);
        let id = records.create("Signature", "Best", None).unwrap();
        records.reload("{ not an array").unwrap_err();
        assert!(matches!(
            records.update(&id, "Signature", "Regards", None),
            Err(RecordError::FileBroken(_))
        ));
        assert!(matches!(
            records.remove(&id),
            Err(RecordError::FileBroken(_))
        ));
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
        assert!(
            on_disk.contains("\nurl = "),
            "quicklink body key: {on_disk}"
        );
    }

    #[test]
    fn a_quicklink_host_follows_its_url() {
        let records = records(Kind::Quicklink);
        let id = records
            .create("Search", "https://example.com?q={{ query }}", None)
            .unwrap();
        assert_eq!(records.host(&id).as_deref(), Some("example.com"));
        records
            .update(
                &id,
                "Search",
                "https://github.com/search?q={{ query }}",
                None,
            )
            .unwrap();
        assert_eq!(records.host(&id).as_deref(), Some("github.com"));
    }

    #[test]
    fn a_quicklink_says_it_needs_a_url() {
        let records = records(Kind::Quicklink);
        let error = records.create("Search", "  ", None).unwrap_err();
        assert_eq!(error.to_string(), "Enter a URL");
    }

    #[test]
    fn a_quicklink_refuses_text_that_cannot_form_a_url() {
        let records = records(Kind::Quicklink);
        let error = records.create("Search", "not a URL", None).unwrap_err();
        assert_eq!(error.to_string(), "Enter a URL such as https://example.com");
        assert!(records.all().unwrap().is_empty());
    }

    #[test]
    fn a_quicklink_accepts_a_url_template() {
        let records = records(Kind::Quicklink);
        records
            .create("Search", "https://example.com?q={{ query }}", None)
            .unwrap();
        assert_eq!(records.all().unwrap().len(), 1);
    }

    const DOCS: &str = "https://example.com/docs";

    #[test]
    fn a_quicklink_browser_survives_reopening_the_store() {
        let dir = std::env::temp_dir().join(format!("dango-records-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let id = Records::open(&dir, Kind::Quicklink)
            .0
            .save(None, "Docs", DOCS, None, Some("firefox"))
            .unwrap();

        let reopened = Records::open(&dir, Kind::Quicklink).0;
        assert_eq!(
            reopened.get(&id).unwrap().browser.as_deref(),
            Some("firefox")
        );
    }

    #[test]
    fn choosing_the_default_browser_removes_the_field() {
        let records = records(Kind::Quicklink);
        let id = records
            .save(None, "Docs", DOCS, None, Some("firefox"))
            .unwrap();

        records
            .save(Some(&id), "Docs", DOCS, None, Some(Browser::DEFAULT_ID))
            .unwrap();

        assert_eq!(records.get(&id).unwrap().browser, None);
        let on_disk = std::fs::read_to_string(records.path()).unwrap();
        assert!(!on_disk.contains("browser"), "no browser field: {on_disk}");
    }

    #[test]
    fn a_browser_this_platform_does_not_offer_is_written_back() {
        let records = records(Kind::Quicklink);
        records
            .reload(&format!(
                "[a]\nname = \"Docs\"\nurl = \"{DOCS}\"\nbrowser = \"safari\"\n"
            ))
            .unwrap();

        records
            .save(Some("a"), "Docs", DOCS, None, Some("safari"))
            .unwrap();

        assert_eq!(records.get("a").unwrap().browser.as_deref(), Some("safari"));
    }

    #[test]
    fn an_edit_without_a_browser_keeps_the_stored_one() {
        let records = records(Kind::Quicklink);
        let id = records
            .save(None, "Docs", DOCS, None, Some("brave"))
            .unwrap();

        records.update(&id, "Docs renamed", DOCS, None).unwrap();

        assert_eq!(records.get(&id).unwrap().browser.as_deref(), Some("brave"));
    }

    #[test]
    fn updating_a_removed_record_says_it_is_gone() {
        let records = records(Kind::Quicklink);
        let error = records.update("missing", "Docs", DOCS, None).unwrap_err();
        assert!(matches!(error, RecordError::Gone));
        assert!(records.all().unwrap().is_empty());
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

    #[test]
    fn a_keyword_that_starts_with_another_is_refused() {
        let records = records(Kind::Snippet);
        records.create("Short", "short", Some(";s")).unwrap();
        let error = records.create("Long", "long", Some(";sig")).unwrap_err();
        assert!(matches!(error, RecordError::KeywordOverlaps(k) if k == ";s"));
    }

    #[test]
    fn a_keyword_that_starts_another_is_refused() {
        let records = records(Kind::Snippet);
        records.create("Long", "long", Some(";sig")).unwrap();
        let error = records.create("Short", "short", Some(";s")).unwrap_err();
        assert!(matches!(error, RecordError::KeywordOverlaps(k) if k == ";sig"));
    }

    #[test]
    fn keywords_differing_in_case_do_not_overlap() {
        let records = records(Kind::Snippet);
        records.create("Short", "short", Some(";s")).unwrap();
        assert!(records.create("Long", "long", Some(";Sig")).is_ok());
    }

    #[test]
    fn an_edit_may_lengthen_its_own_keyword() {
        let records = records(Kind::Snippet);
        let id = records.create("Sig", "sig", Some(";s")).unwrap();
        records.update(&id, "Sig", "sig", Some(";sig")).unwrap();
        assert_eq!(records.get(&id).unwrap().keyword.as_deref(), Some(";sig"));
    }
}

#[cfg(test)]
mod display_tests {
    use super::{Kind, RecordError};
    use crate::templates::TemplateError;

    #[test]
    fn every_fixed_message_is_a_sentence() {
        let errors = [
            RecordError::NoName,
            RecordError::NoBody(Kind::Snippet.missing_body()),
            RecordError::NoBody(Kind::Quicklink.missing_body()),
            RecordError::InvalidUrl,
            RecordError::Gone,
            RecordError::KeywordNeedsPlainSnippet,
            RecordError::KeywordTaken("sig".into()),
            RecordError::KeywordOverlaps("sig".into()),
            RecordError::FileBroken(Kind::Snippet.file_name()),
            RecordError::Write(Kind::Snippet.file_name()),
        ];
        for error in errors {
            let text = error.to_string();
            assert!(text.starts_with(char::is_uppercase), "{text}");
            assert!(!text.ends_with('.') && !text.contains(". "), "{text}");
        }
    }

    #[test]
    fn a_message_carrying_detail_still_starts_with_a_sentence() {
        let errors = [
            RecordError::Parse("TOML parse error at line 1, column 1".into()),
            RecordError::Template(TemplateError::Parse("unexpected end".into())),
        ];
        for error in errors {
            let text = error.to_string();
            assert!(text.starts_with(char::is_uppercase), "{text}");
        }
    }
}
