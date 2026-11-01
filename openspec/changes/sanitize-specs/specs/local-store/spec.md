## MODIFIED Requirements

### Requirement: Store access does not block activation

Opening the database SHALL happen during startup, not on the activation path.
Showing the launcher SHALL NOT wait on a database query.

#### Scenario: Activation is independent of the database

- **WHEN** the user presses the shortcut while a background write is in progress
- **THEN** the launcher opens without waiting for the write

## REMOVED Requirements

### Requirement: Local database in the application data directory

**Reason**: Replaced by "Local database in the per-user data directory". The one location scenario is split into a macOS and a Windows scenario. OpenSpec cannot rename or drop a
scenario inside a MODIFIED block.

**Migration**: None. The rule is unchanged under the new header.


### Requirement: Extension preference values are stored and read back

**Reason**: Preferences are no longer stored in the database. Migration
`0004_drop_config_tables` dropped the preference tables, and preference values
are read from the config file.

**Migration**: Preference values come from the config file (see configuration,
"The file is the source of truth for configuration", which also covers a
value of the wrong type). Declared defaults are in extension-model,
"Extension preferences".

## ADDED Requirements

### Requirement: Local database in the per-user data directory

Dango SHALL persist its data in a single database file in the per-user
application data directory for the platform, created on first run.

#### Scenario: First run creates the database

- **WHEN** Dango starts and no database exists
- **THEN** it is created and the application starts normally

#### Scenario: Location on macOS

- **WHEN** Dango runs on macOS
- **THEN** the database is `dango.sqlite` in
  `~/Library/Application Support/com.nimayer.dango/`

#### Scenario: Location on Windows

- **WHEN** Dango runs on Windows
- **THEN** the database is `dango.sqlite` in `%APPDATA%\com.nimayer.dango\`
