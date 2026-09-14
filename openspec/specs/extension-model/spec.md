# extension-model Specification

## Purpose

The contract every invocable thing in Dango obeys: what an extension declares,
what it may contribute, how enabling and disabling it changes the running
application, and the registry that holds the result.

## Requirements

### Requirement: Versioned extension manifest

Every extension SHALL declare a manifest carrying a `manifestVersion`, a stable
identifier unique across extensions, a display name, and an icon. Dango SHALL
refuse to load an extension whose `manifestVersion` it does not support, and
SHALL continue running without it.

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

### Requirement: Four contribution types

An extension SHALL be able to contribute any combination of four things:
`commands`, a single `rootItems` provider, `services`, and `preferences`. An
extension contributing none of them SHALL be rejected as malformed.

#### Scenario: Extension contributes only commands

- **WHEN** an extension declares commands and nothing else
- **THEN** it loads and those commands appear in the registry

#### Scenario: Extension contributes a service and no commands

- **WHEN** an extension declares only a background service
- **THEN** it loads, the service runs, and no searchable entry is created for it

#### Scenario: Extension contributes nothing

- **WHEN** an extension declares no commands, no root items provider, no services, and no preferences
- **THEN** it is rejected as malformed and is not loaded

### Requirement: Command declaration

Each declared command SHALL carry an identifier unique within its extension, a
title, an invocation mode of either `view` or `no-view`, and optionally a
subtitle, an icon, search keywords, and a user-assigned alias. Its fully
qualified identity SHALL be the extension identifier combined with the command
identifier.

#### Scenario: Command is addressable

- **WHEN** an extension declaring a command is loaded
- **THEN** that command can be resolved by its fully qualified identity

#### Scenario: Keywords widen matching

- **WHEN** a command declares a keyword that does not appear in its title
- **AND** the user types that keyword
- **THEN** the command appears in results

### Requirement: An extension's commands may come from configuration

An extension MAY build its declared commands from the user's configuration
rather than from a fixed set. Such commands SHALL be indistinguishable from
declared ones once registered: they carry the same identity, mode, title, alias,
and keywords, and they are searched, invoked, and hotkey-bound the same way.

When the configuration changes, the extension's command set SHALL be rebuilt and
the registry brought in line without a restart: a command that is gone SHALL be
unregistered and its hotkey released, a new one SHALL be registered, and a
changed one SHALL keep working under its identity.

#### Scenario: Commands are registered from configuration at startup

- **WHEN** the configuration defines commands for such an extension and Dango
  starts
- **THEN** those commands are in the registry and appear in root search

#### Scenario: A command added to the configuration is registered live

- **WHEN** the user adds a command to the configuration while Dango is running
- **THEN** it is registered and searchable without a restart

#### Scenario: A command removed from the configuration is unregistered live

- **WHEN** the user removes such a command from the configuration
- **THEN** it is gone from the registry and from root search
- **AND** any hotkey it held is released

#### Scenario: A malformed command does not take the extension down

- **WHEN** one command in the configuration cannot be built
- **THEN** the extension's other commands are registered normally
- **AND** the problem is surfaced rather than failing silently

#### Scenario: Disabling the extension still removes them

- **WHEN** an extension whose commands come from configuration is disabled
- **THEN** every one of its commands is unregistered, exactly as for a fixed set

### Requirement: Enable and disable is the unit of control

An extension SHALL be enabled or disabled as a whole. Disabling it SHALL
unregister its commands, release its hotkeys, remove its root items provider
from the search pipeline, and stop its services. Enabling SHALL restore all of
them without restarting the application.

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

### Requirement: Typed preferences

An extension SHALL be able to declare preferences at extension level and at
command level, each with a type, a default value, and whether it is required.
Dango SHALL supply the declared default when no value has been set.

#### Scenario: Default is supplied

- **WHEN** a preference has never been set by the user
- **THEN** reading it returns the declared default

#### Scenario: Required preference is missing

- **WHEN** a command declares a required preference with no default and no value is set
- **THEN** invoking that command prompts for the value instead of running

### Requirement: Built-ins are extensions

Built-in features SHALL be declared as extensions using the same manifest,
contribution types, preference schema, and enable and disable lifecycle as any
other extension. They SHALL differ only in that they execute as native code with
full operating system access.

#### Scenario: A built-in can be disabled

- **WHEN** a built-in extension is disabled
- **THEN** its commands disappear from search exactly as a non-built-in's would

#### Scenario: Built-ins are not privileged in the registry

- **WHEN** the registry is queried for a command
- **THEN** the result exposes no distinction between built-in and other extensions beyond which host executes it

### Requirement: A failing extension does not take down the launcher

An error raised while loading, activating, or invoking one extension SHALL NOT
prevent other extensions from working or stop the launcher from opening.

#### Scenario: Extension fails during activation

- **WHEN** one extension raises an error while activating at startup
- **THEN** the launcher opens normally
- **AND** every other extension is active

#### Scenario: Command fails during invocation

- **WHEN** an invoked command raises an error
- **THEN** the failure is shown to the user
- **AND** the launcher remains usable
