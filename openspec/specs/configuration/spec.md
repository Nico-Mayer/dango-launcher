# configuration Specification

## Purpose

Configuring Dango through a single TOML file the user keeps in their dotfiles and
shares over git, rather than a settings interface: what the file controls, where
it lives, how it is loaded and reloaded live, and how a mistake in it is handled
without taking the app down.

## Requirements

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

The top level of the file SHALL hold `version`, the app tables `launcher` and
`hyperkey`, one table per built-in extension, and the reserved `extensions`
table. App tables configure Dango itself rather than a command. `extensions`
is kept for extensions that are not built in. Its table for one extension,
`extensions.<id>`, holds the enabled state, the per-command entries with
aliases and hotkeys, and any other key as that extension's preferences (see
`js-extensions`). The schema SHALL accept any preference key there. No
built-in extension's table SHALL be named `version`, `launcher`, `hyperkey`,
or `extensions`.

#### Scenario: Every built-in has its own table

- **WHEN** the settings reference is generated
- **THEN** each built-in extension is listed under a top-level table named by
  its id without `dango.`

#### Scenario: No extension takes a reserved name

- **WHEN** the built-in extensions are registered
- **THEN** none of their table names is `version`, `launcher`, `hyperkey`, or
  `extensions`

#### Scenario: A preference key is valid

- **WHEN** the file sets `user = "nico"` in `[extensions.gh-prs]`
- **THEN** the file loads without a problem and the value is a preference of
  `gh-prs`

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
