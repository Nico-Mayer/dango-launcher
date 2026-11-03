# Spec Delta

## Purpose

The contract every file the user reads or edits in Dango's config directory
follows: format, place, schema, key style, how named entries are keyed, who
writes the file, how edits apply, and how a bad file is reported, so each file
feels part of one design and a new file has rules to follow.

## ADDED Requirements

### Requirement: Every user file is TOML in the config directory

Every file the user reads or edits SHALL be a TOML file in the config
directory described in `configuration`, honouring `$DANGO_CONFIG_DIR`. The
files SHALL be `config.toml`, `snippets.toml`, `quicklinks.toml`, and
`auth.toml`. Dango SHALL NOT read any other file there as user data.

#### Scenario: The four files

- **WHEN** Dango starts with all four files in the config directory
- **THEN** it reads settings from `config.toml`, snippets from
  `snippets.toml`, quicklinks from `quicklinks.toml`, and keys from
  `auth.toml`

#### Scenario: The override relocates every file

- **WHEN** `$DANGO_CONFIG_DIR` is set
- **THEN** all four files are read from that directory

### Requirement: Each file has a schema an editor can find

Every user file SHALL have a JSON Schema at `.schemas/<name>.schema.json` in
the config directory, where `<name>` is the file's name without `.toml`. A
file Dango creates SHALL begin with `#:schema ./.schemas/<name>.schema.json`,
the comment TOML language servers read to find a schema, and SHALL NOT need a
key to point at it.

#### Scenario: A created file points at its schema

- **WHEN** Dango creates `snippets.toml`
- **THEN** its first line is `#:schema ./.schemas/snippets.schema.json`

#### Scenario: The schemas do not mix with the user's files

- **WHEN** Dango has started
- **THEN** no schema file is in the config directory itself
- **AND** each user file's schema is in `.schemas`

#### Scenario: The editor completes and checks the file

- **WHEN** a user file with its `#:schema` line is opened in an editor whose TOML language server honours `#:schema`
- **THEN** the editor offers the known keys and flags a key the schema rejects

### Requirement: Keys and values follow one style

In every user file, every key Dango defines SHALL be lowercase kebab-case. A
key whose value has a unit SHALL name the unit. A count SHALL be a whole
number. A setting holding several values SHALL be a TOML array, not a
separated string. A value that differs per platform SHALL be one value or a
table with `macos` and `windows` keys.

#### Scenario: Every schema uses kebab-case keys

- **WHEN** the schemas for all four files are generated
- **THEN** every key they name is lowercase kebab-case

### Requirement: An absent key means the default

In every user file, a key left out SHALL mean its built-in default. No file
SHALL have an empty value or placeholder that means "default" or "off".

#### Scenario: A quicklink without a browser

- **WHEN** a quicklink entry has no `browser` key
- **THEN** it opens in the default browser

#### Scenario: A snippet without a keyword

- **WHEN** a snippet entry has no `keyword` key
- **THEN** the snippet has no keyword

### Requirement: Named entries are tables keyed by their id

Entries the user names SHALL be tables or values keyed by their id, never an
array of objects that carry an id field. An id SHALL be lowercase kebab-case
when Dango makes it.

#### Scenario: A quicklink keyed by id

- **WHEN** `quicklinks.toml` holds `[github]` with `name = "GitHub"` and
  `url = "https://github.com"`
- **THEN** a quicklink named GitHub with id `github` is offered

#### Scenario: A key filed under its provider

- **WHEN** `auth.toml` holds `anthropic = "sk-ant-..."`
- **THEN** that is the key for the provider named `anthropic`

### Requirement: Files are versioned and forward compatible

Every user file MAY carry a top-level `version`. In a file whose top-level
keys are ids, `version` SHALL be reserved and SHALL NOT be an id. Keys the
running version does not understand, including settings from a newer Dango and
tables for extensions that are not installed, SHALL be ignored, not treated as
errors, and SHALL be kept when Dango writes the file.

#### Scenario: Unknown keys do not break loading

- **WHEN** a user file contains a key Dango does not recognise
- **THEN** the rest of the file still loads and applies
- **AND** the unknown key is left intact

#### Scenario: The version may be left out

- **WHEN** a user file has no `version`
- **THEN** it loads exactly as it would with one

#### Scenario: Version is not a record

- **WHEN** `snippets.toml` has `version = 1` at the top
- **THEN** no snippet called `version` is offered

### Requirement: Each file has one owner

Each user file SHALL be either user-owned or shared. Dango SHALL NOT change an
existing user-owned file: `config.toml` and `auth.toml`. Its only write to
one SHALL be creating a missing `config.toml` when the user opens it (see
`system-commands`, "Open config file"). Dango MAY write a shared file,
`snippets.toml` and `quicklinks.toml`, and its write SHALL keep the user's
comments, the order of entries, unknown keys, and every entry it did not
change.

#### Scenario: Settings are never written back

- **WHEN** Dango runs with an existing `config.toml` or `auth.toml`, including
  one that does not parse
- **THEN** the file's content is the same as the user last saved it

#### Scenario: A comment survives a write

- **WHEN** `snippets.toml` has a comment above an entry and the user creates
  another snippet from the launcher
- **THEN** the comment is still in the file, above the same entry

#### Scenario: Order survives a write

- **WHEN** the user edits the second of three quicklinks from the launcher
- **THEN** the three entries keep their order in the file

### Requirement: Edits apply live

Dango SHALL watch `config.toml`, `snippets.toml`, and `quicklinks.toml` and
apply a change without a restart. `auth.toml` SHALL be read each time a key
is needed, so a change to it also applies without a restart. Dango's own
writes SHALL NOT be treated as edits to react to.

#### Scenario: Editing the config file re-applies it

- **WHEN** the user edits and saves `config.toml` while Dango is running
- **THEN** the changed settings take effect without restarting Dango

#### Scenario: Editing a record updates the launcher

- **WHEN** the user changes a snippet's template in `snippets.toml` while Dango runs
- **THEN** using that snippet inserts the new text, without a restart

#### Scenario: Deleting an entry removes the record

- **WHEN** the user removes a record's table from its file
- **THEN** that record no longer appears in the launcher

#### Scenario: The app's own write does not loop

- **WHEN** Dango creates `config.toml` or writes `snippets.toml` itself
- **THEN** it does not re-apply its own write as though the user had edited it

### Requirement: A bad file is reported, never fatal

A user file is invalid when it is not valid TOML or a known key has a value of
the wrong type or outside the values it allows. An unreadable or invalid file
SHALL NOT crash Dango. Dango SHALL keep what last loaded cleanly, or the
defaults if nothing has, and SHALL log the file's name, the line, and what is
wrong. A watched file's problem SHALL show in the tray; `ai-providers` says
how a bad `auth.toml` shows. Dango SHALL NOT write a shared file while it is
invalid.

#### Scenario: An invalid config keeps the last good configuration

- **WHEN** the user saves a `config.toml` that does not parse
- **THEN** Dango keeps running on the previously loaded configuration
- **AND** the tray shows a problem and `dango.log` names `config.toml` and the line

#### Scenario: An invalid config at startup falls back to defaults

- **WHEN** Dango starts and the existing `config.toml` does not parse
- **THEN** Dango starts on its built-in defaults
- **AND** the error is surfaced rather than the app refusing to start

#### Scenario: A value of the wrong type makes the file invalid

- **WHEN** the user saves a config with `entries = "many"` in `[clipboard]`,
  or a snippet with `name = 3`
- **THEN** Dango keeps what loaded before
- **AND** the logged error names the file, the setting, and its line

#### Scenario: A value outside the allowed ones makes the file invalid

- **WHEN** the user saves a file with `primary-action = "drop"` in
  `[clipboard]`
- **THEN** Dango keeps running on the previously loaded configuration
- **AND** the logged error names the setting and its line

#### Scenario: A malformed record file keeps the last good records

- **WHEN** the user saves a `quicklinks.toml` that does not parse
- **THEN** the quicklinks from the last good version are still offered
- **AND** the logged error names `quicklinks.toml` and the line

#### Scenario: An invalid shared file is not overwritten

- **WHEN** `snippets.toml` does not parse and the user creates a snippet
- **THEN** the snippet is refused with a message to fix the file first
- **AND** the file is unchanged
