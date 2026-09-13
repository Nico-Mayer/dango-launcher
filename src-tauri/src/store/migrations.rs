use rusqlite::Connection;

pub const MIGRATIONS: &[&str] = &[
    include_str!("migrations/0001_initial.sql"),
    include_str!("migrations/0002_clipboard_history.sql"),
    include_str!("migrations/0003_snippets.sql"),
    include_str!("migrations/0004_drop_config_tables.sql"),
];

#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("database schema version {found} is newer than the supported {supported}")]
    NewerSchema { found: u32, supported: u32 },
    #[error("migration {version} failed: {source}")]
    Apply {
        version: u32,
        #[source]
        source: rusqlite::Error,
    },
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
}

pub fn run(connection: &mut Connection) -> Result<(), MigrationError> {
    run_all(connection, MIGRATIONS)
}

pub fn version(connection: &Connection) -> rusqlite::Result<u32> {
    connection.pragma_query_value(None, "user_version", |row| row.get(0))
}

/// Each migration and its version bump commit together, so a failure leaves
/// the schema exactly at the previous version.
fn run_all(connection: &mut Connection, migrations: &[&str]) -> Result<(), MigrationError> {
    let supported = migrations.len() as u32;
    let found = version(connection)?;
    if found > supported {
        return Err(MigrationError::NewerSchema { found, supported });
    }

    for (index, sql) in migrations.iter().enumerate().skip(found as usize) {
        let version = index as u32 + 1;
        let transaction = connection.transaction()?;
        transaction
            .execute_batch(sql)
            .and_then(|()| transaction.pragma_update(None, "user_version", version))
            .map_err(|source| MigrationError::Apply { version, source })?;
        transaction.commit()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table_exists(connection: &Connection, name: &str) -> bool {
        connection
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [name],
                |row| row.get::<_, i64>(0),
            )
            .unwrap()
            == 1
    }

    #[test]
    fn applies_every_migration_and_records_the_version() {
        let mut connection = Connection::open_in_memory().unwrap();
        run(&mut connection).unwrap();
        assert_eq!(version(&connection).unwrap(), MIGRATIONS.len() as u32);
        for table in ["frecency", "local_app_index", "local_clipboard_history"] {
            assert!(table_exists(&connection, table), "{table} missing");
        }
        // The configuration tables were retired by migration 0004.
        for table in ["extension_state", "preferences"] {
            assert!(!table_exists(&connection, table), "{table} should be gone");
        }
    }

    /// Upgrading a database that predates a migration must add only what the
    /// migration adds, leaving what the user already has alone.
    #[test]
    fn an_existing_database_upgrades_without_disturbing_its_tables() {
        let mut connection = Connection::open_in_memory().unwrap();
        run_all(&mut connection, &MIGRATIONS[..1]).unwrap();
        connection
            .execute(
                "INSERT INTO frecency (id, item_id, launch_count, last_launched_at, updated_at) \
                 VALUES ('a', 'ext.cmd', 5, 1, 1)",
                [],
            )
            .unwrap();
        assert!(!table_exists(&connection, "local_clipboard_history"));

        run(&mut connection).unwrap();

        assert!(table_exists(&connection, "local_clipboard_history"));
        let kept: i64 = connection
            .query_row(
                "SELECT launch_count FROM frecency WHERE item_id = 'ext.cmd'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(kept, 5, "the upgrade must not touch existing rows");
    }

    #[test]
    fn running_twice_applies_nothing_new() {
        let mut connection = Connection::open_in_memory().unwrap();
        run(&mut connection).unwrap();
        run(&mut connection).unwrap();
        assert_eq!(version(&connection).unwrap(), MIGRATIONS.len() as u32);
    }

    #[test]
    fn refuses_a_newer_schema() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection.pragma_update(None, "user_version", 99).unwrap();
        let error = run(&mut connection).unwrap_err();
        assert!(matches!(
            error,
            MigrationError::NewerSchema {
                found: 99,
                supported
            } if supported == MIGRATIONS.len() as u32
        ));
    }

    #[test]
    fn a_failing_migration_leaves_the_previous_version() {
        let mut connection = Connection::open_in_memory().unwrap();
        let migrations = [
            "CREATE TABLE first (x);",
            "CREATE TABLE second (x); THIS IS NOT SQL;",
        ];
        let error = run_all(&mut connection, &migrations).unwrap_err();
        assert!(matches!(error, MigrationError::Apply { version: 2, .. }));
        assert_eq!(version(&connection).unwrap(), 1);
        assert!(table_exists(&connection, "first"));
        assert!(!table_exists(&connection, "second"));
    }
}
