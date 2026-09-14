## MODIFIED Requirements

### Requirement: The file is the source of truth for configuration

The configuration file SHALL be where the launcher hotkey, each extension's
enabled state, each extension's preference values, command aliases, command
hotkeys, and application hotkeys come from. A value present in the file SHALL
take precedence over the built-in default; a value absent from the file SHALL
fall back to the default.

Configuration SHALL be addressed by extension id and, where it applies, command
id or application name, so every setting has one stable path and new settings
extend the same tree.

#### Scenario: A preference set in the file is used

- **WHEN** the file sets a preference for an extension and that extension reads it
- **THEN** the value from the file is returned rather than the declared default

#### Scenario: An extension disabled in the file is disabled

- **WHEN** the file marks an extension disabled and Dango starts
- **THEN** that extension's commands, root items, and services are not active

#### Scenario: An absent value falls back to the default

- **WHEN** the file does not mention a preference
- **THEN** reading it returns the extension's declared default

#### Scenario: An application hotkey has one path

- **WHEN** the file binds an application under
  `extensions.dango.applications.apps.<name>.hotkey`
- **THEN** that binding is applied
- **AND** writing the file back preserves it alongside every other setting
