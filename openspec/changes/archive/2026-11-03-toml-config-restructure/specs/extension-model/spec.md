# Spec Delta

## RENAMED Requirements

- FROM: `### Requirement: Four contribution types`
- TO: `### Requirement: Three contribution types`

## MODIFIED Requirements

### Requirement: Three contribution types

An extension SHALL be able to contribute any combination of three things:
`commands`, a single `rootItems` provider, and `services`. An extension
contributing none of them SHALL be rejected as malformed. Settings are not a
contribution: a built-in extension's settings are its table in the
configuration file, described in `openspec/specs/configuration`.

#### Scenario: Extension contributes only commands

- **WHEN** an extension declares commands and nothing else
- **THEN** it loads and those commands appear in the registry

#### Scenario: Extension contributes a service and no commands

- **WHEN** an extension declares only a background service
- **THEN** it loads, the service runs, and no searchable entry is created for it

#### Scenario: Extension contributes nothing

- **WHEN** an extension declares no commands, no root items provider, and no services
- **THEN** it is rejected as malformed and is not loaded

### Requirement: Built-ins are extensions

Built-in features SHALL be declared as extensions using the same manifest,
contribution types, and enable and disable lifecycle as any other extension.
They SHALL differ only in that they execute as native code with full operating
system access, and in that their settings are typed tables of the
configuration file.

#### Scenario: A built-in can be disabled

- **WHEN** a built-in extension is disabled
- **THEN** its commands disappear from search exactly as a non-built-in's would

#### Scenario: Built-ins are not privileged in the registry

- **WHEN** the registry is queried for a command
- **THEN** the result exposes no distinction between built-in and other extensions beyond which host executes it

### Requirement: A command can be disabled on its own

A command whose entry in the configuration file at `<table>.commands.<id>`
sets `enabled` to false SHALL be disabled, for every extension, built-in or
not. `<table>` is the built-in's own table, or `extensions.<id>` for one that
is not built in. A disabled command SHALL NOT appear in root search and SHALL
NOT be invocable. What happens to its hotkey is in command-hotkeys. The
extension's other commands, its root items, and its services SHALL be
unaffected. An absent `enabled` SHALL mean enabled. A change to a command's
`enabled` value SHALL apply when the file is reloaded, without a restart.

#### Scenario: A disabled command leaves root search

- **WHEN** the file sets `enabled` to false on a command of an enabled extension
- **THEN** that command does not appear in root search for any query
- **AND** the extension's other commands still appear

#### Scenario: A command disabled after results were produced

- **WHEN** the user confirms a command that was disabled in the file since the results were produced
- **THEN** nothing is invoked
- **AND** the launcher stays open and reports that the command is unavailable

#### Scenario: Disabling a command does not touch the rest of the extension

- **WHEN** a command of an extension with a running background service or a root items provider is disabled
- **THEN** the service keeps running and the root items still appear

#### Scenario: Turning a command back on without restart

- **WHEN** the user removes `enabled = false` from a command, or sets it to true, while Dango is running
- **THEN** the command appears in root search and its hotkey works again
- **AND** the application was not restarted

## REMOVED Requirements

### Requirement: Extension preferences

**Reason**: Every built-in now reads its settings from a typed table in the
configuration file. The types, defaults, allowed values, and descriptions live
with that table and reach the schema and the settings reference from there, so
a separate preference declaration in the manifest only repeats them.

**Migration**: None needed, Dango is alpha. A built-in that declared
preferences moves them into its settings table. Settings for extensions that
are not built in are designed together with the extension host.
