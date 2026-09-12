//! The user's snippets, and the same shape reused for quicklinks.
//!
//! Both are a name plus a template the user wrote, both are created and edited
//! from the launcher, and both should exist on either of the author's machines.
//! So one store serves both, differing only in which table it writes and what
//! the template column is called.

use std::sync::Arc;

use crate::store::Store;
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
    #[error(transparent)]
    Store(#[from] rusqlite::Error),
}

/// One snippet or quicklink as the store hands it back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub id: String,
    pub name: String,
    /// The template text: a snippet's body, or a quicklink's URL.
    pub body: String,
}

/// Which of the two tables a store instance owns. The tables are identical in
/// shape, so the only real difference is the word the user sees when something
/// is missing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Snippet,
    Quicklink,
}

impl Kind {
    fn table(self) -> &'static str {
        match self {
            Kind::Snippet => "snippets",
            Kind::Quicklink => "quicklinks",
        }
    }

    fn body_column(self) -> &'static str {
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
    store: Arc<Store>,
    kind: Kind,
}

impl Records {
    pub fn new(store: Arc<Store>, kind: Kind) -> Self {
        Self { store, kind }
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// Refuses anything that cannot be used later: no name, no body, or a
    /// template that will not parse. A template is checked here, where the user
    /// is still editing it, rather than when they reach for it in a hurry.
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

    pub fn create(&self, name: &str, body: &str, now: i64) -> Result<String, RecordError> {
        self.validate(name, body)?;
        let id = uuid::Uuid::new_v4().to_string();
        let sql = format!(
            "INSERT INTO {} (id, name, {}, created_at, updated_at, deleted_at) \
             VALUES (?1, ?2, ?3, ?4, ?4, NULL)",
            self.kind.table(),
            self.kind.body_column()
        );
        self.store.with(|c| {
            c.execute(&sql, rusqlite::params![id, name.trim(), body, now])?;
            Ok(())
        })?;
        Ok(id)
    }

    pub fn update(&self, id: &str, name: &str, body: &str, now: i64) -> Result<(), RecordError> {
        self.validate(name, body)?;
        let sql = format!(
            "UPDATE {} SET name = ?2, {} = ?3, updated_at = ?4 \
             WHERE id = ?1 AND deleted_at IS NULL",
            self.kind.table(),
            self.kind.body_column()
        );
        let changed = self
            .store
            .with(|c| c.execute(&sql, rusqlite::params![id, name.trim(), body, now]))?;
        if changed == 0 {
            return Err(RecordError::Gone);
        }
        Ok(())
    }

    /// Soft delete, because the column exists for sync and a hard delete would
    /// come back from the other machine.
    pub fn remove(&self, id: &str, now: i64) -> Result<(), RecordError> {
        let sql = format!(
            "UPDATE {} SET deleted_at = ?2, updated_at = ?2 \
             WHERE id = ?1 AND deleted_at IS NULL",
            self.kind.table()
        );
        let changed = self
            .store
            .with(|c| c.execute(&sql, rusqlite::params![id, now]))?;
        if changed == 0 {
            return Err(RecordError::Gone);
        }
        Ok(())
    }

    pub fn all(&self) -> Result<Vec<Record>, RecordError> {
        let sql = format!(
            "SELECT id, name, {} FROM {} WHERE deleted_at IS NULL ORDER BY name COLLATE NOCASE",
            self.kind.body_column(),
            self.kind.table()
        );
        let records = self.store.with(|c| {
            let mut stmt = c.prepare(&sql)?;
            let rows = stmt.query_map([], |row| {
                Ok(Record {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    body: row.get(2)?,
                })
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })?;
        Ok(records)
    }

    pub fn get(&self, id: &str) -> Result<Record, RecordError> {
        self.all()?
            .into_iter()
            .find(|record| record.id == id)
            .ok_or(RecordError::Gone)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn records(kind: Kind) -> Records {
        Records::new(Arc::new(Store::in_memory().unwrap()), kind)
    }

    #[test]
    fn a_snippet_is_stored_and_read_back() {
        let records = records(Kind::Snippet);
        let id = records.create("Signature", "Best, Nico", 1).unwrap();

        let all = records.all().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, id);
        assert_eq!(all[0].name, "Signature");
        assert_eq!(all[0].body, "Best, Nico");
    }

    #[test]
    fn a_snippet_is_edited_and_the_old_version_is_not_kept() {
        let records = records(Kind::Snippet);
        let id = records.create("Signature", "Best, Nico", 1).unwrap();

        records.update(&id, "Sign-off", "Regards, Nico", 2).unwrap();

        let all = records.all().unwrap();
        assert_eq!(all.len(), 1, "an edit is not a second snippet");
        assert_eq!(all[0].name, "Sign-off");
        assert_eq!(all[0].body, "Regards, Nico");
    }

    #[test]
    fn a_removed_snippet_is_gone_from_the_list() {
        let records = records(Kind::Snippet);
        let id = records.create("Signature", "Best, Nico", 1).unwrap();

        records.remove(&id, 2).unwrap();

        assert!(records.all().unwrap().is_empty());
        assert!(matches!(records.get(&id), Err(RecordError::Gone)));
    }

    #[test]
    fn removal_is_soft_so_sync_cannot_resurrect_it() {
        let records = records(Kind::Snippet);
        let id = records.create("Signature", "Best, Nico", 1).unwrap();
        records.remove(&id, 2).unwrap();

        let still_there: i64 = records
            .store
            .with(|c| c.query_row("SELECT COUNT(*) FROM snippets", [], |row| row.get(0)))
            .unwrap();
        assert_eq!(still_there, 1, "the row stays, carrying deleted_at");
    }

    #[test]
    fn a_snippet_needs_a_name() {
        let records = records(Kind::Snippet);
        assert!(matches!(
            records.create("   ", "Best, Nico", 1),
            Err(RecordError::NoName)
        ));
        assert!(records.all().unwrap().is_empty());
    }

    #[test]
    fn a_snippet_needs_some_text() {
        let records = records(Kind::Snippet);
        assert!(matches!(
            records.create("Signature", "", 1),
            Err(RecordError::NoBody(_))
        ));
        assert!(records.all().unwrap().is_empty());
    }

    #[test]
    fn a_template_that_will_not_parse_is_refused_where_it_is_written() {
        let records = records(Kind::Snippet);
        let error = records.create("Broken", "unclosed {{ name", 1).unwrap_err();
        assert!(matches!(error, RecordError::Template(_)));
        assert!(records.all().unwrap().is_empty());
    }

    #[test]
    fn editing_something_that_is_gone_says_so() {
        let records = records(Kind::Snippet);
        assert!(matches!(
            records.update("nope", "Name", "Body", 1),
            Err(RecordError::Gone)
        ));
    }

    #[test]
    fn quicklinks_use_their_own_table() {
        let store = Arc::new(Store::in_memory().unwrap());
        let snippets = Records::new(store.clone(), Kind::Snippet);
        let quicklinks = Records::new(store, Kind::Quicklink);

        snippets.create("Signature", "Best, Nico", 1).unwrap();
        quicklinks
            .create("Search", "https://example.com?q={{ query }}", 1)
            .unwrap();

        assert_eq!(snippets.all().unwrap().len(), 1);
        assert_eq!(quicklinks.all().unwrap().len(), 1);
        assert_eq!(
            quicklinks.all().unwrap()[0].body,
            "https://example.com?q={{ query }}"
        );
    }

    #[test]
    fn a_quicklink_says_it_needs_a_url() {
        let records = records(Kind::Quicklink);
        let error = records.create("Search", "  ", 1).unwrap_err();
        assert_eq!(error.to_string(), "give it a URL");
    }

    #[test]
    fn records_come_back_in_name_order_regardless_of_case() {
        let records = records(Kind::Snippet);
        records.create("zebra", "z", 1).unwrap();
        records.create("Apple", "a", 2).unwrap();
        records.create("mango", "m", 3).unwrap();

        let names: Vec<_> = records
            .all()
            .unwrap()
            .into_iter()
            .map(|record| record.name)
            .collect();
        assert_eq!(names, vec!["Apple", "mango", "zebra"]);
    }
}
