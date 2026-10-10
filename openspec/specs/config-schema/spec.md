# config-schema Specification

## Purpose

A JSON Schema for the config file, generated from the code that reads the
file, written beside it so an editor can complete and check settings against
the running version, and a settings reference generated from the same model so
documentation cannot drift from behaviour.

## Requirements

### Requirement: The schema is generated from the code

Dango SHALL produce a JSON Schema for the config file from its own definition
of the file: the typed settings, every built-in extension's id, its preference
keys with their type, default, allowed values, and description, and its
shipped command ids. The schema SHALL NOT be maintained by hand anywhere.

#### Scenario: A preference is known by key

- **WHEN** a built-in extension declares a preference
- **THEN** the schema names that key under that extension's `preferences`, with its type, its default, its description, and its allowed values when it has any

#### Scenario: A shipped command is known by id

- **WHEN** a built-in extension ships a command
- **THEN** the schema names that command id under that extension's `commands`

#### Scenario: A setting's description comes from the code

- **WHEN** a setting on the config structs carries a description in the code
- **THEN** the schema carries that same text for the setting

#### Scenario: The schema cannot disagree with the parser

- **WHEN** a hotkey token is accepted by the hotkey parser
- **THEN** the schema's hotkey pattern accepts a chord using it
- **AND** a chord with a token the parser rejects does not match the pattern

### Requirement: The schema is strict about known objects

Every object the schema describes SHALL reject keys it does not name, except
maps whose keys are the user's own names: applications bound to hotkeys, AI
providers, and AI commands. The `extensions` map SHALL be closed to the
built-in extension ids. Enumerated settings SHALL list their values.

#### Scenario: A misspelt key is rejected

- **WHEN** a config contains `hotkeys` where `hotkey` is meant
- **THEN** the schema reports the key as not allowed

#### Scenario: A user-named entry is accepted

- **WHEN** a config binds an application, defines a provider, or defines an AI command under a name of the user's choosing
- **THEN** the schema accepts the entry and checks its contents

#### Scenario: A command setting that only one extension reads

- **WHEN** a config gives a `prompt` to a command of an extension other than `dango.ai`
- **THEN** the schema rejects the key for that extension

#### Scenario: An enumerated value is checked

- **WHEN** a config sets a provider `kind`, a command's `thinking` or `output`, or the clipboard's `primary-action` to a value Dango does not have
- **THEN** the schema rejects it and lists the values it does have

#### Scenario: The runtime stays lenient

- **WHEN** a config contains a key the schema rejects
- **THEN** Dango still loads the file and preserves the key on write-back, exactly as before

### Requirement: The schema is written beside the config file

At startup Dango SHALL write the schema to `config.schema.json` in the config
directory, honouring `$DANGO_CONFIG_DIR`, replacing an older copy whose content
differs and leaving an identical copy untouched. The write SHALL NOT be
treated as a config edit, and a failure to write it SHALL be logged and SHALL
NOT affect startup.

#### Scenario: First start writes the schema

- **WHEN** Dango starts and no `config.schema.json` exists in the config directory
- **THEN** the file exists afterwards and describes the running version's config

#### Scenario: A newer build replaces the schema

- **WHEN** Dango starts and the existing `config.schema.json` differs from what this build generates
- **THEN** the file is overwritten with this build's schema

#### Scenario: An identical schema is not rewritten

- **WHEN** Dango starts and the existing `config.schema.json` already matches
- **THEN** the file is not written again

#### Scenario: The schema write is not a config edit

- **WHEN** the schema file is written while Dango is running
- **THEN** the config is not reloaded and the config status does not change

#### Scenario: The schema cannot be written

- **WHEN** the config directory refuses the write
- **THEN** Dango starts normally
- **AND** `dango.log` says the schema file could not be written and why

### Requirement: An editor can find the schema

A config file Dango creates SHALL begin with a `$schema` key whose value is
the relative path `./config.schema.json`, and Dango SHALL keep that key when
it writes the file back.

#### Scenario: A created file points at the schema

- **WHEN** Dango creates a config file
- **THEN** its first key is `$schema` with the value `./config.schema.json`

#### Scenario: The key survives a write-back

- **WHEN** a config file has a `$schema` key and Dango writes the file
- **THEN** the key and its value are still present afterwards

#### Scenario: The editor completes and checks the file

- **WHEN** a config file with the `$schema` key is opened in an editor whose JSON language server honours `$schema`
- **THEN** the editor offers the known keys and flags a key the schema rejects

### Requirement: The settings reference is generated and kept current

Dango's repository SHALL carry a settings reference, `docs/config.md`,
generated from the same model as the schema, listing every setting path with
its type, default, allowed values, and description. A test SHALL fail when
the committed reference differs from what the code generates, and SHALL say
how to regenerate it.

#### Scenario: Every setting is listed

- **WHEN** the reference is generated
- **THEN** every property the schema names appears with its path, type, default, allowed values, and description

#### Scenario: A stale reference fails the tests

- **WHEN** a setting is added or its description changed and the reference is not regenerated
- **THEN** the test suite fails naming `docs/config.md` and the command that regenerates it

#### Scenario: Regeneration makes the test pass

- **WHEN** the reference is regenerated with the named command
- **THEN** the test passes with no other change
