# local-store Specification

## Purpose

Local persistence for everything Dango remembers between runs, with a schema
that can evolve safely and a record convention that keeps optional sync possible
without building any of it now.

## Requirements

### Requirement: Local database in the application data directory

Dango SHALL persist its data in a single database file in the per-user
application data directory for the platform, created on first run.

#### Scenario: First run creates the database

- **WHEN** Dango starts and no database exists
- **THEN** it is created and the application starts normally

#### Scenario: Location is per-user

- **WHEN** Dango runs on macOS or on Windows
- **THEN** the database is stored under that platform's per-user application data directory

### Requirement: Forward-only migrations

The schema SHALL be versioned and migrated forward automatically at startup.
Migrations SHALL be applied in order and each SHALL be applied at most once.

#### Scenario: Upgrade applies pending migrations

- **WHEN** Dango starts against a database at an older schema version
- **THEN** the pending migrations run in order and the application starts

#### Scenario: Migration failure is not partial

- **WHEN** a migration fails partway through
- **THEN** the database is left at the previous version
- **AND** the failure is reported rather than the application continuing against a broken schema

#### Scenario: Database is newer than the application

- **WHEN** the database schema version is newer than the running application supports
- **THEN** the application reports the mismatch rather than migrating backwards or corrupting data

### Requirement: Syncable record convention

Every table holding data a user could ever want on a second machine SHALL carry
a UUID primary key, an `updated_at` timestamp, and a nullable `deleted_at`
timestamp. Deletion of such records SHALL be a soft delete.

#### Scenario: Records carry sync metadata

- **WHEN** a syncable record is created
- **THEN** it has a UUID identifier and an `updated_at` timestamp

#### Scenario: Update refreshes the timestamp

- **WHEN** a syncable record is modified
- **THEN** its `updated_at` is advanced

#### Scenario: Delete is soft

- **WHEN** a syncable record is deleted
- **THEN** its `deleted_at` is set
- **AND** it no longer appears in normal queries

### Requirement: Non-syncable data is separated

Data that is machine-local by nature, such as the application index and cached
icons, SHALL be stored so that it is distinguishable from syncable data and can
be rebuilt from scratch.

#### Scenario: Local cache can be discarded

- **WHEN** the machine-local data is deleted and Dango restarts
- **THEN** it is rebuilt automatically
- **AND** no user-created data is lost

### Requirement: Store access does not block activation

Opening the database and reading settings SHALL happen during startup, not on
the activation path. Showing the launcher SHALL NOT wait on a database query.

#### Scenario: Activation is independent of the database

- **WHEN** the user presses the shortcut while a background write is in progress
- **THEN** the launcher still meets its activation budget

### Requirement: Corrupted database is recoverable

If the database cannot be opened because it is corrupt, Dango SHALL still start,
SHALL report the problem, and SHALL preserve the unreadable file rather than
overwriting it.

#### Scenario: Corruption on startup

- **WHEN** the database file is corrupt at startup
- **THEN** Dango starts with an empty store
- **AND** the corrupt file is preserved
- **AND** the user is told what happened

### Requirement: Preference values are stored and read back

The store SHALL persist a preference value against the extension and, where the
preference is command-scoped, the command that declares it. Reading a preference
that has never been set SHALL yield the declared default rather than an absence.

#### Scenario: Value is stored and read back

- **WHEN** a preference is set and later read
- **THEN** the stored value is returned

#### Scenario: Unset preference yields its default

- **WHEN** a preference that has never been set is read
- **THEN** the default declared in the manifest is returned

#### Scenario: Extension and command scopes are separate

- **WHEN** an extension-level preference and a command-level preference share a key
- **THEN** each reads back its own value

#### Scenario: Value survives a restart

- **WHEN** Dango restarts after a preference was set
- **THEN** reading it returns the value that was set

#### Scenario: Stored value no longer fits its declared type

- **WHEN** a stored value cannot be read as the type the manifest declares
- **THEN** the declared default is returned
- **AND** the extension still loads
