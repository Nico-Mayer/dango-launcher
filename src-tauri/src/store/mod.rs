mod migrations;

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OpenFlags};

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
