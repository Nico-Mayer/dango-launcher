## MODIFIED Requirements

### Requirement: The file is the source of truth for configuration

The configuration file SHALL be where the launcher hotkey, each extension's
enabled state, each extension's preference values, command aliases, command
hotkeys, and application hotkeys come from. It SHALL also be where commands
themselves are defined for an extension whose commands come from configuration,
including their titles and whatever else that extension needs to run them. A
value present in the file SHALL take precedence over the built-in default; a
value absent from the file SHALL fall back to the default.

Configuration SHALL be addressed by extension id and, where it applies, command
id or application name, so every setting has one stable path and new settings
extend the same tree.

Credentials SHALL NOT live in this file. They live beside it, in their own file,
so the configuration stays shareable.

#### Scenario: A preference set in the file is used

- **WHEN** the file sets a preference for an extension and that extension reads it
- **THEN** the value from the file is returned rather than the declared default

#### Scenario: An extension disabled in the file is disabled

- **WHEN** the file marks an extension disabled and Dango starts
- **THEN** that extension's commands, root items, and services are not active

#### Scenario: An absent value falls back to the default

- **WHEN** the file does not mention a preference
- **THEN** reading it returns the extension's declared default

#### Scenario: A value of the wrong type falls back to the default

- **WHEN** the file sets a preference to a value that cannot be read as the
  type the extension declares
- **THEN** reading it returns the declared default
- **AND** the extension still loads

#### Scenario: An application hotkey has one path

- **WHEN** the file binds an application under
  `extensions.dango.applications.apps.<name>.hotkey`
- **THEN** that binding is applied

#### Scenario: A command defined in the file has one path

- **WHEN** the file defines a command under
  `extensions.<extension-id>.commands.<command-id>`, with its title and the
  settings that extension reads
- **THEN** that command is offered by the extension
- **AND** its alias and hotkey are read from the same entry

#### Scenario: No credential is written to the file

- **WHEN** Dango creates the configuration file
- **THEN** no key or secret appears in it

### Requirement: Edits apply live

Dango SHALL watch the configuration file and re-apply it when it changes, without
a restart: the launcher hotkey, enabled state, preferences, and aliases SHALL
reflect the new file. When Dango itself creates the file, that write SHALL NOT
be treated as an external edit to react to.

#### Scenario: Editing the file re-applies it

- **WHEN** the user edits and saves `config.json` while Dango is running
- **THEN** the changed settings take effect without restarting Dango

#### Scenario: The app's own write does not loop

- **WHEN** Dango creates the file itself
- **THEN** it does not re-apply its own write as though the user had edited it

## REMOVED Requirements

### Requirement: The file survives being written back

**Reason**: Dango never writes the configuration file back. The only write is
creating a missing file, which system-commands
("Open config file") covers.

**Migration**: None. A future settings editor brings its own write-back rules.

## ADDED Requirements

### Requirement: Dango writes the file only to create it

Dango SHALL NOT change an existing configuration file. The only write SHALL be
creating a missing file when the user opens it (see system-commands, "Open
config file").

#### Scenario: Settings are never written back

- **WHEN** Dango runs with an existing `config.json`, including one that does
  not parse
- **THEN** the file's content is the same as the user last saved it
