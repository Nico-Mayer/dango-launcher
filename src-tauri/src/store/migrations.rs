use rusqlite::Connection;

pub const MIGRATIONS: &[&str] = &[include_str!("migrations/0001_initial.sql")];

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
        for table in [
            "extension_state",
            "preferences",
            "frecency",
            "local_app_index",
        ] {
            assert!(table_exists(&connection, table), "{table} missing");
        }
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
