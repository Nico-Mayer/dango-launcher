## MODIFIED Requirements

### Requirement: Versioned extension manifest

Every extension SHALL declare a manifest carrying a `manifestVersion`, a stable
identifier unique across extensions, a display name, and optionally an icon.
Dango SHALL refuse to load an extension whose `manifestVersion` it does not
support, and SHALL continue running without it.

#### Scenario: Manifest version is supported

- **WHEN** an extension declares a supported `manifestVersion`
- **THEN** it loads and its contributions are registered

#### Scenario: Manifest version is not supported

- **WHEN** an extension declares a `manifestVersion` newer than Dango supports
- **THEN** the extension is not loaded
- **AND** the reason is recorded where a developer can read it
- **AND** all other extensions load normally

#### Scenario: Identifiers collide

- **WHEN** two extensions declare the same identifier
- **THEN** the conflict is reported and only one is registered

### Requirement: Enable and disable is the unit of control

An extension SHALL be enabled or disabled as a whole. Disabling it SHALL
unregister its commands, release its hotkeys, remove its root items provider
from the search pipeline, and stop its services. Enabling SHALL restore all of
them without restarting the application. Turning off a single command of an
enabled extension is narrower and SHALL affect only that command, as the
requirement "A command can be disabled on its own" defines.

#### Scenario: Disabling removes commands from search

- **WHEN** an enabled extension's commands are matching a query and the extension is disabled
- **THEN** those commands no longer appear for that query

#### Scenario: Disabling stops services

- **WHEN** an extension with a running background service is disabled
- **THEN** the service stops and releases the resources it held

#### Scenario: Re-enabling restores contributions without restart

- **WHEN** a disabled extension is enabled again
- **THEN** its commands, root items, and services are active again
- **AND** the application was not restarted

#### Scenario: Enabled state survives restart

- **WHEN** an extension is disabled and the application is restarted
- **THEN** the extension is still disabled

## REMOVED Requirements

### Requirement: A single command can be disabled

**Reason**: Its hotkey scenarios repeated command-hotkeys. The rest moves to
"A command can be disabled on its own" without them.

**Migration**: What happens to a disabled command's hotkey, and its log line,
is in command-hotkeys.

## ADDED Requirements

### Requirement: A command can be disabled on its own

A command whose entry in the configuration file at
`extensions.<id>.commands.<id>` sets `enabled` to false SHALL be disabled, for
every extension, built-in or not. A disabled command SHALL NOT appear in root
search and SHALL NOT be invocable. What happens to its hotkey is in
command-hotkeys. The extension's other commands, its root items, and its
services SHALL be unaffected. An absent `enabled` SHALL mean enabled. A change
to a command's `enabled` value SHALL apply when the file is reloaded, without a
restart.

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

- **WHEN** the user removes `enabled: false` from a command, or sets it to true, while Dango is running
- **THEN** the command appears in root search and its hotkey works again
- **AND** the application was not restarted
