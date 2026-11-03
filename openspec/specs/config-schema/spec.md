# config-schema Specification

## Purpose

A JSON Schema for the config file, generated from the code that reads the
file, written beside it so an editor can complete and check settings against
the running version, and a settings reference generated from the same model so
documentation cannot drift from behaviour.

## Requirements

### Requirement: The schema is generated from the code

Dango SHALL produce a JSON Schema for every user file named in
`config-files` from its own definition of that file. For the config file this
covers the typed settings of the app tables, every built-in extension's table
with its settings' types, defaults, allowed values, and descriptions, and its
shipped command ids. For the snippet, quicklink, and key files it covers the
fields of an entry with their types and descriptions. No schema SHALL be
maintained by hand anywhere.

#### Scenario: A preference is known by key

- **WHEN** a built-in extension has a setting
- **THEN** the schema names that key in that extension's table, with its type, its default, its description, and its allowed values when it has any

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

#### Scenario: A record's fields are known

- **WHEN** the snippets schema is generated
- **THEN** it names `name`, `template`, and `keyword` for an entry, with their
  types and descriptions

#### Scenario: A quicklink's browser lists its values

- **WHEN** the quicklinks schema is generated
- **THEN** `browser` lists the browser ids Dango knows

### Requirement: The schema is strict about known objects

Every table a schema describes SHALL reject keys it does not name, except
tables whose keys are the user's own names: applications bound to hotkeys, AI
providers, AI commands, the entries of the reserved `extensions` table, and
the top level of the snippet, quicklink, and key files. The config file's top
level SHALL be closed to `version`, the app tables, the built-in extension
tables, and `extensions`. Enumerated settings SHALL list their values.

#### Scenario: A misspelt key is rejected

- **WHEN** a config contains `hotkeys` where `hotkey` is meant in `[launcher]`
- **THEN** the schema reports the key as not allowed

#### Scenario: An unknown top-level table is rejected

- **WHEN** a config contains a `[clipbaord]` table
- **THEN** the schema reports the table as not allowed

#### Scenario: A user-named entry is accepted

- **WHEN** a config binds an application, defines a provider, or defines an AI command under a name of the user's choosing
- **THEN** the schema accepts the entry and checks its contents

#### Scenario: A command setting that only one extension reads

- **WHEN** a config gives a `prompt` to a command in a table other than `[ai]`
- **THEN** the schema rejects the key for that table

#### Scenario: An enumerated value is checked

- **WHEN** a config sets a provider `kind`, a command's `thinking` or `output`, or the clipboard's `primary-action` to a value Dango does not have
- **THEN** the schema rejects it and lists the values it does have

#### Scenario: A misspelt record field is rejected

- **WHEN** a snippet entry sets `templte` instead of `template`
- **THEN** the schema reports the key as not allowed

#### Scenario: Any record id is accepted

- **WHEN** `quicklinks.toml` holds an entry under an id of the user's choosing
- **THEN** the schema accepts it and checks its fields

#### Scenario: A key must be text

- **WHEN** `auth.toml` sets a provider's key to a number
- **THEN** the schema reports the value as not a string

#### Scenario: The runtime stays lenient

- **WHEN** a user file contains a key the schema rejects as unknown
- **THEN** Dango still loads the file, exactly as before

### Requirement: The settings reference is generated and kept current

Dango's repository SHALL carry a settings reference, `docs/config.md`,
generated from the same model as the schemas, listing every user file and
every setting or field path in it with its type, default, allowed values, and
description. A test SHALL fail when the committed reference differs from what
the code generates, and SHALL say how to regenerate it.

#### Scenario: Every setting is listed

- **WHEN** the reference is generated
- **THEN** every property the schemas name appears with its path, type, default, allowed values, and description

#### Scenario: Every file is listed

- **WHEN** the reference is generated
- **THEN** it has a section for each of `config.toml`, `snippets.toml`,
  `quicklinks.toml`, and `auth.toml`

#### Scenario: A stale reference fails the tests

- **WHEN** a setting is added or its description changed and the reference is not regenerated
- **THEN** the test suite fails naming `docs/config.md` and the command that regenerates it

#### Scenario: Regeneration makes the test pass

- **WHEN** the reference is regenerated with the named command
- **THEN** the test passes with no other change

### Requirement: The hotkey pattern follows the modifier rule

The schema's hotkey pattern SHALL accept exactly the chords the hotkey parser
accepts: one or more modifiers followed by one key, or a function key from F1
to F20 alone. The settings reference SHALL state the same rule.

#### Scenario: A bare letter does not match

- **WHEN** a config sets a hotkey to `a` or `space`
- **THEN** the schema reports the value as not matching the hotkey pattern
- **AND** the parser refuses the same value

#### Scenario: A lone function key matches

- **WHEN** a config sets a hotkey to `f1` or `f20`
- **THEN** the schema accepts the value
- **AND** the parser accepts the same value

#### Scenario: A function key past F20 does not match

- **WHEN** a config sets a hotkey to `f21` or `ctrl+f24`
- **THEN** the schema reports the value as not matching the hotkey pattern
- **AND** the parser refuses the same value

#### Scenario: The reference states the rule

- **WHEN** the settings reference is generated
- **THEN** its hotkey section says a chord needs a modifier unless its key is F1 to F20

### Requirement: The schemas are written to the schemas folder

At startup Dango SHALL write each user file's schema to the path
`config-files` names for it, honouring `$DANGO_CONFIG_DIR`, creating the
`.schemas` folder if needed, replacing a copy whose content differs, and
leaving an identical copy untouched. A failed write SHALL be logged and SHALL
NOT affect startup.

#### Scenario: First start writes the schemas

- **WHEN** Dango starts and there is no `.schemas` folder in the config directory
- **THEN** the folder exists afterwards with `config.schema.json`,
  `snippets.schema.json`, `quicklinks.schema.json`, and `auth.schema.json`,
  each describing the running version's file

#### Scenario: A newer build replaces a schema

- **WHEN** Dango starts and a schema in `.schemas` differs from what this build generates
- **THEN** that file is overwritten with this build's schema

#### Scenario: An identical schema is not rewritten

- **WHEN** Dango starts and a schema in `.schemas` already matches
- **THEN** that file is not written again

#### Scenario: The schemas cannot be written

- **WHEN** the config directory refuses the write
- **THEN** Dango starts normally
- **AND** `dango.log` says which schema could not be written and why
