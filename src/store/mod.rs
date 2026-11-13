mod migrations;

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OpenFlags, OptionalExtension};

pub use migrations::MigrationError;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("could not create data directory {path}: {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("could not open database: {0}")]
    Open(#[from] rusqlite::Error),
    #[error(transparent)]
    Migrate(#[from] MigrationError),
    #[error("could not move corrupt database {path} aside: {source}")]
    SetAside {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// Input, extension id, item id, use count, last used at.
pub type InputHistoryRow = (String, String, String, u32, i64);

pub struct Store {
    connection: Mutex<Connection>,
}

pub struct Opened {
    pub store: Store,
    /// Where the unreadable previous database was moved, when there was one.
    pub recovered_from: Option<PathBuf>,
}

impl Store {
    pub fn open(path: &Path) -> Result<Opened, StoreError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|source| StoreError::CreateDir {
                path: dir.to_path_buf(),
                source,
            })?;
        }
        match Self::open_file(path) {
            Ok(store) => Ok(Opened {
                store,
                recovered_from: None,
            }),
            Err(error) if is_corruption(&error) => {
                let aside = set_aside(path)?;
                Ok(Opened {
                    store: Self::open_file(path)?,
                    recovered_from: Some(aside),
                })
            }
            Err(error) => Err(error),
        }
    }

    #[cfg(test)]
    pub fn in_memory() -> Result<Self, StoreError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn open_file(path: &Path) -> Result<Self, StoreError> {
        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        Self::from_connection(Connection::open_with_flags(path, flags)?)
    }

    fn from_connection(mut connection: Connection) -> Result<Self, StoreError> {
        connection.pragma_update(None, "foreign_keys", true)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "NORMAL")?;
        migrations::run(&mut connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn schema_version(&self) -> rusqlite::Result<u32> {
        self.with(migrations::version)
    }

    pub fn with<T>(
        &self,
        query: impl FnOnce(&Connection) -> rusqlite::Result<T>,
    ) -> rusqlite::Result<T> {
        let connection = self.connection.lock().unwrap();
        query(&connection)
    }

    /// A JavaScript extension's stored value, as the JSON it was stored as.
    pub fn extension_value(
        &self,
        extension_id: &str,
        key: &str,
    ) -> rusqlite::Result<Option<String>> {
        self.with(|connection| {
            connection
                .query_row(
                    "SELECT value_json FROM local_extension_storage
                     WHERE extension_id = ?1 AND key = ?2",
                    rusqlite::params![extension_id, key],
                    |row| row.get(0),
                )
                .optional()
        })
    }

    pub fn set_extension_value(
        &self,
        extension_id: &str,
        key: &str,
        value_json: &str,
    ) -> rusqlite::Result<()> {
        self.with(|connection| {
            connection.execute(
                "INSERT INTO local_extension_storage (extension_id, key, value_json, updated_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (extension_id, key)
                 DO UPDATE SET value_json = excluded.value_json, updated_at = excluded.updated_at",
                rusqlite::params![extension_id, key, value_json, now_millis()],
            )?;
            Ok(())
        })
    }

    pub fn remove_extension_value(&self, extension_id: &str, key: &str) -> rusqlite::Result<()> {
        self.with(|connection| {
            connection.execute(
                "DELETE FROM local_extension_storage WHERE extension_id = ?1 AND key = ?2",
                rusqlite::params![extension_id, key],
            )?;
            Ok(())
        })
    }

    pub fn extension_keys(&self, extension_id: &str) -> rusqlite::Result<Vec<String>> {
        self.with(|connection| {
            let mut statement = connection.prepare(
                "SELECT key FROM local_extension_storage WHERE extension_id = ?1 ORDER BY key",
            )?;
            let keys = statement
                .query_map([extension_id], |row| row.get(0))?
                .collect();
            keys
        })
    }

    pub fn load_apps(&self) -> rusqlite::Result<Vec<(String, String, Option<String>)>> {
        self.with(|c| {
            let mut stmt =
                c.prepare("SELECT app_id, name, target FROM local_app_index ORDER BY name")?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            })?;
            rows.collect()
        })
    }

    /// Replaces the whole index in one transaction, so a reader never sees a
    /// half-written index.
    pub fn replace_apps(&self, apps: &[(String, String, Option<String>)]) -> rusqlite::Result<()> {
        let now = now_millis();
        self.with(|c| {
            let tx = c.unchecked_transaction()?;
            tx.execute("DELETE FROM local_app_index", [])?;
            {
                let mut stmt = tx.prepare(
                    "INSERT INTO local_app_index (app_id, name, target, indexed_at) \
                     VALUES (?1, ?2, ?3, ?4)",
                )?;
                for (id, name, target) in apps {
                    stmt.execute(params![id, name, target, now])?;
                }
            }
            tx.commit()
        })
    }

    pub fn load_frecency(&self) -> rusqlite::Result<Vec<(String, String, u32, i64)>> {
        self.with(|c| {
            let mut stmt = c.prepare(
                "SELECT extension_id, item_id, launch_count, last_launched_at \
                 FROM frecency WHERE deleted_at IS NULL",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)? as u32,
                    row.get::<_, i64>(3)?,
                ))
            })?;
            rows.collect()
        })
    }

    pub fn save_frecency(
        &self,
        extension_id: &str,
        item_id: &str,
        launch_count: u32,
        last_launched_at: i64,
    ) -> rusqlite::Result<()> {
        let now = now_millis();
        self.with(|c| {
            c.execute(
                "INSERT INTO frecency \
                 (id, extension_id, item_id, launch_count, last_launched_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
                 ON CONFLICT(extension_id, item_id) DO UPDATE SET \
                 launch_count = ?4, last_launched_at = ?5, updated_at = ?6, deleted_at = NULL",
                params![
                    uuid::Uuid::new_v4().to_string(),
                    extension_id,
                    item_id,
                    launch_count as i64,
                    last_launched_at,
                    now
                ],
            )?;
            Ok(())
        })
    }

    pub fn load_input_history(&self) -> rusqlite::Result<Vec<InputHistoryRow>> {
        self.with(|c| {
            let mut stmt = c.prepare(
                "SELECT input, extension_id, item_id, use_count, last_used_at \
                 FROM input_history WHERE deleted_at IS NULL",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)? as u32,
                    row.get::<_, i64>(4)?,
                ))
            })?;
            rows.collect()
        })
    }

    pub fn save_input_history(
        &self,
        input: &str,
        extension_id: &str,
        item_id: &str,
        use_count: u32,
        last_used_at: i64,
    ) -> rusqlite::Result<()> {
        let now = now_millis();
        self.with(|c| {
            c.execute(
                "INSERT INTO input_history \
                 (id, input, extension_id, item_id, use_count, last_used_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
                 ON CONFLICT(input, extension_id, item_id) DO UPDATE SET \
                 use_count = ?5, last_used_at = ?6, updated_at = ?7, deleted_at = NULL",
                params![
                    uuid::Uuid::new_v4().to_string(),
                    input,
                    extension_id,
                    item_id,
                    use_count as i64,
                    last_used_at,
                    now
                ],
            )?;
            Ok(())
        })
    }
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn is_corruption(error: &StoreError) -> bool {
    use rusqlite::ffi::ErrorCode::{DatabaseCorrupt, NotADatabase};
    let sqlite = match error {
        StoreError::Open(e) => e,
        StoreError::Migrate(
            MigrationError::Sqlite(e) | MigrationError::Apply { source: e, .. },
        ) => e,
        _ => return false,
    };
    matches!(
        sqlite,
        rusqlite::Error::SqliteFailure(ffi, _) if matches!(ffi.code, DatabaseCorrupt | NotADatabase)
    )
}

fn set_aside(path: &Path) -> Result<PathBuf, StoreError> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut aside = path.as_os_str().to_owned();
    aside.push(format!(".corrupt-{stamp}"));
    let aside = PathBuf::from(aside);
    std::fs::rename(path, &aside).map_err(|source| StoreError::SetAside {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(aside)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db() -> PathBuf {
        std::env::temp_dir()
            .join(format!("dango-store-{}", uuid::Uuid::new_v4()))
            .join("dango.sqlite")
    }

    fn cleanup(path: &Path) {
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn an_extension_value_survives_reopening() {
        let path = temp_db();
        {
            let store = Store::open(&path).unwrap().store;
            store
                .set_extension_value("gh-prs", "token", r#""a""#)
                .unwrap();
            store
                .set_extension_value("gh-prs", "token", r#""b""#)
                .unwrap();
        }
        let store = Store::open(&path).unwrap().store;

        assert_eq!(
            store.extension_value("gh-prs", "token").unwrap().as_deref(),
            Some(r#""b""#)
        );
        assert_eq!(store.extension_keys("gh-prs").unwrap(), vec!["token"]);
        store.remove_extension_value("gh-prs", "token").unwrap();
        assert_eq!(store.extension_value("gh-prs", "token").unwrap(), None);
        drop(store);
        cleanup(&path);
    }

    #[test]
    fn extensions_keep_their_own_values() {
        let store = Store::in_memory().unwrap();
        store.set_extension_value("a", "key", "1").unwrap();
        store.set_extension_value("b", "key", "2").unwrap();

        assert_eq!(
            store.extension_value("a", "key").unwrap().as_deref(),
            Some("1")
        );
        assert_eq!(
            store.extension_value("b", "key").unwrap().as_deref(),
            Some("2")
        );
    }

    #[test]
    fn first_run_creates_the_database_and_migrates() {
        let path = temp_db();
        let opened = Store::open(&path).unwrap();
        assert!(path.exists());
        assert!(opened.recovered_from.is_none());
        assert_eq!(
            opened.store.schema_version().unwrap(),
            migrations::MIGRATIONS.len() as u32
        );
        cleanup(&path);
    }

    #[test]
    fn a_database_file_uses_the_write_ahead_log() {
        let path = temp_db();
        let store = Store::open(&path).unwrap().store;
        let mode: String = store
            .with(|c| c.pragma_query_value(None, "journal_mode", |row| row.get(0)))
            .unwrap();
        assert_eq!(mode, "wal");
        drop(store);
        cleanup(&path);
    }

    #[test]
    fn corrupt_file_is_moved_aside_and_replaced() {
        let path = temp_db();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"this is not a database").unwrap();

        let opened = Store::open(&path).unwrap();
        let aside = opened.recovered_from.expect("corrupt file set aside");

        assert_eq!(
            std::fs::read(&aside).unwrap(),
            b"this is not a database",
            "the unreadable file must be preserved untouched"
        );
        assert!(opened.store.schema_version().unwrap() > 0);
        cleanup(&path);
    }

    #[test]
    fn input_history_pairs_round_trip_and_update_in_place() {
        let store = Store::in_memory().unwrap();
        store
            .save_input_history("cal", "apps", "calendar", 1, 10)
            .unwrap();
        store
            .save_input_history("cal", "apps", "calculator", 1, 11)
            .unwrap();
        store
            .save_input_history("cal", "other", "calendar", 1, 12)
            .unwrap();
        store
            .save_input_history("cal", "apps", "calendar", 2, 13)
            .unwrap();

        let mut loaded = store.load_input_history().unwrap();
        loaded.sort();
        assert_eq!(
            loaded,
            vec![
                ("cal".into(), "apps".into(), "calculator".into(), 1, 11),
                ("cal".into(), "apps".into(), "calendar".into(), 2, 13),
                ("cal".into(), "other".into(), "calendar".into(), 1, 12),
            ]
        );
    }

    #[test]
    fn frecency_round_trips_and_updates_in_place() {
        let store = Store::in_memory().unwrap();
        store.save_frecency("apps", "calendar", 1, 10).unwrap();
        store.save_frecency("apps", "safari", 4, 11).unwrap();
        store.save_frecency("other", "calendar", 3, 12).unwrap();
        store.save_frecency("apps", "calendar", 2, 13).unwrap();

        let mut loaded = store.load_frecency().unwrap();
        loaded.sort();
        assert_eq!(
            loaded,
            vec![
                ("apps".into(), "calendar".into(), 2, 13),
                ("apps".into(), "safari".into(), 4, 11),
                ("other".into(), "calendar".into(), 3, 12),
            ]
        );
    }

    #[test]
    fn a_reindex_replaces_every_app() {
        let store = Store::in_memory().unwrap();
        let app = |id: &str| (id.to_string(), id.to_uppercase(), None);
        store.replace_apps(&[app("old"), app("kept")]).unwrap();
        store.replace_apps(&[app("kept"), app("new")]).unwrap();

        let ids: Vec<String> = store
            .load_apps()
            .unwrap()
            .into_iter()
            .map(|a| a.0)
            .collect();
        assert_eq!(ids, ["kept", "new"]);
    }

    #[test]
    fn a_failed_reindex_rolls_back_and_frees_the_connection() {
        let store = Store::in_memory().unwrap();
        let app = |id: &str| (id.to_string(), id.to_uppercase(), None);
        store.replace_apps(&[app("kept")]).unwrap();

        assert!(store.replace_apps(&[app("dup"), app("dup")]).is_err());

        assert!(store.with(|c| Ok(c.is_autocommit())).unwrap());
        let ids: Vec<String> = store
            .load_apps()
            .unwrap()
            .into_iter()
            .map(|a| a.0)
            .collect();
        assert_eq!(ids, ["kept"]);
    }

    #[test]
    fn in_memory_store_is_migrated() {
        let store = Store::in_memory().unwrap();
        assert_eq!(
            store.schema_version().unwrap(),
            migrations::MIGRATIONS.len() as u32
        );
    }
}
