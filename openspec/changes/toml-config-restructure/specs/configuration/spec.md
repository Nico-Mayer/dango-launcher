# Spec Delta

## RENAMED Requirements

- FROM: `### Requirement: Configuration lives in one JSON file`
- TO: `### Requirement: Configuration lives in one TOML file`

## MODIFIED Requirements

### Requirement: Configuration lives in one TOML file

Dango SHALL read its configuration from a single TOML file named `config.toml`
in a fixed directory, `~/.config/dango/` on both platforms, and SHALL honour a
`$DANGO_CONFIG_DIR` environment override for that directory. The directory SHALL
be created if it does not exist. A `config.json` in that directory SHALL NOT be
read.

The file SHALL be optional: with no file present, Dango runs entirely on its
built-in defaults. The schema Dango writes beside the file is not
configuration and SHALL NOT count as the file being present.

#### Scenario: No file means defaults

- **WHEN** Dango starts and no `config.toml` exists
- **THEN** it runs normally with every setting at its built-in default
- **AND** it does not create a config file the user did not ask for

#### Scenario: The override relocates the directory

- **WHEN** `$DANGO_CONFIG_DIR` is set and contains a `config.toml`
- **THEN** Dango reads configuration from there rather than `~/.config/dango/`

#### Scenario: An old JSON file is not read

- **WHEN** the config directory holds a `config.json` and no `config.toml`
- **THEN** Dango runs on its built-in defaults
- **AND** the `config.json` is left untouched

### Requirement: Configuration is versioned and forward compatible

The file MAY carry a top-level `version`. Keys the running version does not
understand, including settings from a newer Dango and tables for extensions
that are not installed, SHALL be preserved and ignored rather than treated as
errors, so an older Dango and a newer one can share one file.

#### Scenario: Unknown keys do not break loading

- **WHEN** the file contains a key Dango does not recognise
- **THEN** the rest of the configuration still loads and applies
- **AND** the unknown key is left intact for whatever does understand it

#### Scenario: The version may be left out

- **WHEN** the file has no `version`
- **THEN** it loads exactly as it would with one

### Requirement: Edits apply live

Dango SHALL watch the configuration file and re-apply it when it changes, without
a restart: the launcher hotkey, enabled state, settings, and aliases SHALL
reflect the new file. When Dango itself creates the file, that write SHALL NOT
be treated as an external edit to react to.

#### Scenario: Editing the file re-applies it

- **WHEN** the user edits and saves `config.toml` while Dango is running
- **THEN** the changed settings take effect without restarting Dango

#### Scenario: The app's own write does not loop

- **WHEN** Dango creates the file itself
- **THEN** it does not re-apply its own write as though the user had edited it

### Requirement: A bad file never takes the app down

An unreadable or invalid configuration file SHALL NOT crash Dango or leave it
unconfigured. A file is invalid when it is not valid TOML, or when a known
setting has a value of the wrong type or outside the values it allows. Dango
SHALL keep running on the last configuration that loaded cleanly, or on the
built-in defaults if none has, and SHALL surface the problem, with the line it
is on, where the user can find it rather than failing silently.

#### Scenario: Invalid file keeps the last good configuration

- **WHEN** the user saves a `config.toml` that does not parse
- **THEN** Dango keeps running on the previously loaded configuration
- **AND** the error is surfaced through the tray and written to a log

#### Scenario: Invalid file at startup falls back to defaults

- **WHEN** Dango starts and the existing `config.toml` does not parse
- **THEN** Dango starts on its built-in defaults
- **AND** the error is surfaced rather than the app refusing to start

#### Scenario: A value of the wrong type makes the file invalid

- **WHEN** the user saves a file with `entries = "many"` in `[clipboard]`
- **THEN** Dango keeps running on the previously loaded configuration
- **AND** the surfaced error names the setting and its line

#### Scenario: A value outside the allowed ones makes the file invalid

- **WHEN** the user saves a file with `primary-action = "drop"` in
  `[clipboard]`
- **THEN** Dango keeps running on the previously loaded configuration
- **AND** the surfaced error names the setting and its line

### Requirement: Dango writes the file only to create it

Dango SHALL NOT change an existing configuration file. The only write SHALL be
creating a missing file when the user opens it (see system-commands, "Open
config file").

#### Scenario: Settings are never written back

- **WHEN** Dango runs with an existing `config.toml`, including one that does
  not parse
- **THEN** the file's content is the same as the user last saved it

## ADDED Requirements

### Requirement: The file is the source of truth for every setting

The configuration file SHALL be where every setting comes from: the launcher
hotkey, each extension's enabled state and settings, command aliases and
hotkeys, application hotkeys, and the commands an extension builds from
configuration. A value in the file SHALL take precedence over the built-in
default. Every setting SHALL have one stable path. Credentials SHALL NOT live
in this file, so it stays shareable.

#### Scenario: A setting in the file is used

- **WHEN** the file sets `entries = 100` in `[clipboard]`
- **THEN** the clipboard history keeps at most 100 entries rather than the
  default

#### Scenario: An extension disabled in the file is disabled

- **WHEN** the file sets `enabled = false` in an extension's table and Dango
  starts
- **THEN** that extension's commands, root items, and services are not active

#### Scenario: An absent value falls back to the default

- **WHEN** the file does not mention a setting
- **THEN** the setting has its built-in default

#### Scenario: An application hotkey has one path

- **WHEN** the file binds an application under
  `applications.hotkeys.<name>.hotkey`
- **THEN** that binding is applied

#### Scenario: A command defined in the file has one path

- **WHEN** the file defines a command under `<table>.commands.<command-id>`,
  with its title and the settings that extension reads
- **THEN** that command is offered by the extension
- **AND** its alias and hotkey are read from the same entry

#### Scenario: No credential is written to the file

- **WHEN** Dango creates the configuration file
- **THEN** no key or secret appears in it

### Requirement: The top level holds app tables and extension tables

The top level of the file SHALL hold `version`, the app tables `launcher`,
`hyperkey`, and `selection`, one table per built-in extension, and the
reserved `extensions` table. App tables configure Dango itself rather than a
command. `extensions` is kept for extensions that are not built in, and holds
only what Dango itself reads for them: enabled state, aliases, and hotkeys. No
built-in extension's table SHALL be named `version`, `launcher`, `hyperkey`,
`selection`, or `extensions`.

#### Scenario: Every built-in has its own table

- **WHEN** the settings reference is generated
- **THEN** each built-in extension is listed under a top-level table named by
  its id without `dango.`

#### Scenario: No extension takes a reserved name

- **WHEN** the built-in extensions are registered
- **THEN** none of their table names is `version`, `launcher`, `hyperkey`,
  `selection`, or `extensions`

### Requirement: Inside a table, two keys are reserved

In every extension table, `enabled` SHALL turn the extension on or off and
`commands` SHALL hold per-command entries keyed by command id. No other
setting SHALL use either name. Any table that can be turned off SHALL do it
with `enabled`, app tables included.

#### Scenario: An extension's own setting never shadows a reserved key

- **WHEN** the built-in extensions' settings are listed
- **THEN** no extension has a setting of its own named `enabled` or `commands`

#### Scenario: The hyperkey is turned on the same way

- **WHEN** the file sets `enabled = true` in `[hyperkey]`
- **THEN** the hyperkey is on

### Requirement: Keys and values follow one style

Every key SHALL be lowercase kebab-case. A key whose value has a unit SHALL
name the unit, as in `reset-after-seconds` or `entry-megabytes`. A count SHALL
be a whole number. A setting holding several values SHALL be a TOML array, not
a separated string. A value that differs per platform SHALL be written as one
value or as a table with `macos` and `windows` keys. A path from an extension
table to a value SHALL be at most three keys deep, as in
`ai.commands.<id>.prompt`.

#### Scenario: A list is an array

- **WHEN** the file sets `excluded-applications = ["1Password", "Bitwarden"]`
  in `[clipboard]`
- **THEN** both applications are excluded

#### Scenario: A separated string is not a list

- **WHEN** the file sets `excluded-applications = "1Password, Bitwarden"` in
  `[clipboard]`
- **THEN** the file is invalid and the error names the setting

#### Scenario: A per-platform value

- **WHEN** an application entry sets `name = { macos = "Brave Browser",
  windows = "Brave" }`
- **THEN** each platform looks for its own name

#### Scenario: A camelCase key is not recognised

- **WHEN** a provider entry sets `baseUrl` instead of `base-url`
- **THEN** the key is ignored as unknown
- **AND** the schema reports it as not allowed

### Requirement: An absent key always means the default

A key left out SHALL mean the setting's built-in default, for every setting.
The file SHALL have no empty value or placeholder that means "default" or
"off". `enabled` left out SHALL mean the table's default: on for an extension,
off for the hyperkey.

#### Scenario: An extension table without enabled

- **WHEN** `[clipboard]` sets settings but not `enabled`
- **THEN** the clipboard extension runs

#### Scenario: A hyperkey table without enabled

- **WHEN** `[hyperkey]` sets `tap = "escape"` but not `enabled`
- **THEN** the hyperkey is off

## REMOVED Requirements

### Requirement: The file is the source of truth for configuration

**Reason**: Replaced by "The file is the source of truth for every setting".
Settings now live in one table per built-in extension rather than under
`extensions.<id>`, and a value of the wrong type no longer falls back to its
default: it makes the file invalid (see "A bad file never takes the app down").

**Migration**: None, Dango is alpha. The author rewrites their file as
`config.toml`.
