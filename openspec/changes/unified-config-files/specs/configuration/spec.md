# Spec Delta

## MODIFIED Requirements

### Requirement: Configuration lives in one TOML file

Dango SHALL read its configuration from a single TOML file named `config.toml`
in a fixed directory, `~/.config/dango/` on both platforms, and SHALL honour a
`$DANGO_CONFIG_DIR` environment override for that directory. The directory SHALL
be created if it does not exist. The file follows the contract in
`config-files`.

The file SHALL be optional: with no file present, Dango runs entirely on its
built-in defaults.

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

### Requirement: Keys and values follow one style

The config file SHALL follow the key and value style in `config-files`. A path
from an extension table to a value SHALL be at most three keys deep, as in
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

A key left out of the config file SHALL mean its default, as `config-files`
sets for every file. `enabled` left out SHALL mean the table's default: on
for an extension, off for the hyperkey.

#### Scenario: An extension table without enabled

- **WHEN** `[clipboard]` sets settings but not `enabled`
- **THEN** the clipboard extension runs

#### Scenario: A hyperkey table without enabled

- **WHEN** `[hyperkey]` sets `tap = "escape"` but not `enabled`
- **THEN** the hyperkey is off

## REMOVED Requirements

### Requirement: Configuration is versioned and forward compatible

**Reason**: Holds for every user file; now in `config-files`, "Files are
versioned and forward compatible".

**Migration**: None. The config file behaves the same.

### Requirement: Edits apply live

**Reason**: Holds for every watched user file; now in `config-files`, "Edits
apply live".

**Migration**: None. The config file behaves the same.

### Requirement: A bad file never takes the app down

**Reason**: Holds for every user file; now in `config-files`, "A bad file is
reported, never fatal". The log line now names `config.toml`.

**Migration**: None.

### Requirement: Dango writes the file only to create it

**Reason**: Now in `config-files`, "Each file has one owner", which covers
`auth.toml` too.

**Migration**: None. The config file behaves the same.
