# Spec Delta

## MODIFIED Requirements

### Requirement: Three contribution types

An extension SHALL be able to contribute any combination of three things:
`commands`, a single `rootItems` provider, and `services`. An extension
contributing none of them SHALL be rejected as malformed. Settings are not a
contribution: a built-in extension's settings are its table in the
configuration file, described in `openspec/specs/configuration`. Only built-in
extensions SHALL contribute `services`. The `rootItems` of a JavaScript
extension SHALL be cached rows, as `js-extensions` defines, not a provider
asked on every query.

#### Scenario: Extension contributes only commands

- **WHEN** an extension declares commands and nothing else
- **THEN** it loads and those commands appear in the registry

#### Scenario: Extension contributes a service and no commands

- **WHEN** an extension declares only a background service
- **THEN** it loads, the service runs, and no searchable entry is created for it

#### Scenario: Extension contributes nothing

- **WHEN** an extension declares no commands, no root items provider, and no services
- **THEN** it is rejected as malformed and is not loaded

#### Scenario: A JavaScript extension declares a service

- **WHEN** a JavaScript extension's manifest declares `services`
- **THEN** it is not loaded and the problem is reported

### Requirement: Built-ins are extensions

Built-in features SHALL be declared as extensions using the same manifest,
contribution types, and enable and disable lifecycle as any other extension.
They SHALL differ only in that they execute as native code with full operating
system access, in that their settings are typed tables of the configuration
file, and in that only they contribute services. JavaScript extensions, which
`js-extensions` defines, run in an embedded JavaScript runtime and keep their
settings under `extensions.<id>`.

#### Scenario: A built-in can be disabled

- **WHEN** a built-in extension is disabled
- **THEN** its commands disappear from search exactly as a non-built-in's would

#### Scenario: Built-ins are not privileged in the registry

- **WHEN** the registry is queried for a command
- **THEN** the result exposes no distinction between built-in and other extensions beyond which host executes it

#### Scenario: A JavaScript extension is disabled

- **WHEN** the config file sets `enabled = false` in `[extensions.gh-prs]`
- **THEN** the commands and root items of the JavaScript extension `gh-prs`
  leave root search, exactly as a built-in's would
