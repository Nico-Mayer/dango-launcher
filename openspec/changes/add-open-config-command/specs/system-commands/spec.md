## MODIFIED Requirements

### Requirement: System commands are searchable and invocable

The `system` extension SHALL contribute lock screen, sleep, empty trash, quit
application, and open config file as commands, searchable in root search like
any other command.

#### Scenario: Command is found by name

- **WHEN** the user types part of a system command's title
- **THEN** that command appears in the results

#### Scenario: Command is found by keyword

- **WHEN** the user types a keyword a system command declares but that is absent from its title
- **THEN** that command appears in the results

#### Scenario: Extension can be disabled

- **WHEN** the `system` extension is disabled
- **THEN** none of its commands appear in any query

## ADDED Requirements

### Requirement: Open config file

Open config file SHALL be a no-view command, titled `Open Config File`, that
opens the config file the configuration spec names, honouring
`$DANGO_CONFIG_DIR`, with the application the operating system associates with
`.json` files. Dango SHALL NOT choose, bundle, or configure an editor. The
command SHALL be found by the words a user reaches for when looking for
settings, including "config", "settings", "preferences", "keybinds", and
"hotkeys".

When no config file exists, the command SHALL create one containing only the
schema version before opening it, and the creation SHALL NOT be treated as an
external edit to react to. An existing file SHALL be opened as it is, with no
change to its content.

#### Scenario: Found by a settings word

- **WHEN** the user types "settings", "preferences", "keybinds", or "hotkeys"
- **THEN** Open Config File appears in the results

#### Scenario: The file opens on macOS

- **WHEN** open config file is invoked on macOS and the config file exists
- **THEN** the file opens in the application macOS associates with `.json` files
- **AND** the launcher hides

#### Scenario: The file opens on Windows

- **WHEN** open config file is invoked on Windows and the config file exists
- **THEN** the file opens in the application Windows associates with `.json` files
- **AND** the launcher hides

#### Scenario: Windows has no association for the file type

- **WHEN** open config file is invoked on Windows and no application is associated with `.json` files
- **THEN** the Windows chooser for picking an application appears, as it does for any such file
- **AND** the launcher hides, because the request reached the operating system

#### Scenario: No config file exists yet

- **WHEN** open config file is invoked and there is no config file
- **THEN** a config file containing only the schema version is created in the config directory
- **AND** that file is opened
- **AND** the config status does not report a reload, because the write was Dango's own

#### Scenario: An existing file is left untouched

- **WHEN** open config file is invoked and the config file exists, including one that does not parse
- **THEN** the file's content is the same after the command as before
- **AND** the file is opened

#### Scenario: The directory override is honoured

- **WHEN** `$DANGO_CONFIG_DIR` is set and open config file is invoked
- **THEN** the file in that directory is created if needed and opened, not the one under `~/.config/dango/`

#### Scenario: The file cannot be opened

- **WHEN** the operating system cannot start the handler for the file
- **THEN** the launcher stays open
- **AND** the banner says the config file couldn't be opened and names `dango.log`
- **AND** the operating system's own error text is written to the log, not shown

#### Scenario: The file cannot be created

- **WHEN** there is no config file and it cannot be written, for example because the directory is read-only
- **THEN** the launcher stays open
- **AND** the banner says the config file couldn't be created and names `dango.log`
- **AND** nothing is opened

#### Scenario: Bound to a hotkey

- **WHEN** the command has a hotkey under `extensions.dango.system.commands.open-config.hotkey` and the user presses that chord while another application is focused
- **THEN** the config file opens without the launcher appearing
