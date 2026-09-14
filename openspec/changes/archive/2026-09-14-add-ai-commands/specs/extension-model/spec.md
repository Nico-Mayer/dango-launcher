## ADDED Requirements

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
