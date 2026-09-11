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

    /// Enabled unless a row says otherwise, so a never-seen extension defaults
    /// to on.
    pub fn extension_enabled(&self, extension_id: &str) -> rusqlite::Result<bool> {
        self.with(|c| {
            c.query_row(
                "SELECT enabled FROM extension_state \
                 WHERE extension_id = ?1 AND deleted_at IS NULL",
                [extension_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map(|value| value.is_none_or(|enabled| enabled != 0))
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
            c.execute_batch("BEGIN; DELETE FROM local_app_index;")?;
            {
                let mut stmt = c.prepare(
                    "INSERT INTO local_app_index (app_id, name, target, indexed_at) \
                     VALUES (?1, ?2, ?3, ?4)",
                )?;
                for (id, name, target) in apps {
                    stmt.execute(params![id, name, target, now])?;
                }
            }
            c.execute_batch("COMMIT;")?;
            Ok(())
        })
    }

    pub fn delete_app(&self, app_id: &str) -> rusqlite::Result<()> {
        self.with(|c| {
            c.execute("DELETE FROM local_app_index WHERE app_id = ?1", [app_id])?;
            Ok(())
        })
    }

    pub fn load_frecency(&self) -> rusqlite::Result<Vec<(String, u32, i64)>> {
        self.with(|c| {
            let mut stmt = c.prepare(
                "SELECT item_id, launch_count, last_launched_at \
                 FROM frecency WHERE deleted_at IS NULL",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)? as u32,
                    row.get::<_, i64>(2)?,
                ))
            })?;
            rows.collect()
        })
    }

    pub fn save_frecency(
        &self,
        item_id: &str,
        launch_count: u32,
        last_launched_at: i64,
    ) -> rusqlite::Result<()> {
        let now = now_millis();
        self.with(|c| {
            c.execute(
                "INSERT INTO frecency \
                 (id, item_id, launch_count, last_launched_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5) \
                 ON CONFLICT(item_id) DO UPDATE SET \
                 launch_count = ?3, last_launched_at = ?4, updated_at = ?5, deleted_at = NULL",
                params![
                    uuid::Uuid::new_v4().to_string(),
                    item_id,
                    launch_count as i64,
                    last_launched_at,
                    now
                ],
            )?;
            Ok(())
        })
    }

    pub fn set_extension_enabled(&self, extension_id: &str, enabled: bool) -> rusqlite::Result<()> {
        let now = now_millis();
        self.with(|c| {
            c.execute(
                "INSERT INTO extension_state (id, extension_id, enabled, updated_at) \
                 VALUES (?1, ?2, ?3, ?4) \
                 ON CONFLICT(extension_id) DO UPDATE SET \
                 enabled = ?3, updated_at = ?4, deleted_at = NULL",
                params![
                    uuid::Uuid::new_v4().to_string(),
                    extension_id,
                    enabled as i64,
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
    fn in_memory_store_is_migrated() {
        let store = Store::in_memory().unwrap();
        assert_eq!(
            store.schema_version().unwrap(),
            migrations::MIGRATIONS.len() as u32
        );
    }
}
