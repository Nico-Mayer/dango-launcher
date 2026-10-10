# system-commands Specification

## Purpose

The built-in extension contributing the everyday actions a launcher is
expected to have: locking the screen, putting the machine to sleep, emptying
the trash, quitting a running application, and opening Dango's own config
file.

## Requirements

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

### Requirement: Lock screen

Lock screen SHALL be a no-view command that locks the session immediately,
leaving running applications untouched.

#### Scenario: Lock on macOS

- **WHEN** lock screen is invoked on macOS
- **THEN** the session locks and the login prompt is shown

#### Scenario: Lock on Windows

- **WHEN** lock screen is invoked on Windows
- **THEN** the session locks and the lock screen is shown

#### Scenario: Launcher gets out of the way

- **WHEN** lock screen is invoked
- **THEN** the launcher hides before the screen locks, so it is not on screen when the session is unlocked

### Requirement: Sleep

Sleep SHALL be a no-view command that puts the machine into its normal sleep
state.

#### Scenario: Sleep on macOS

- **WHEN** sleep is invoked on macOS
- **THEN** the machine enters sleep

#### Scenario: Sleep on Windows

- **WHEN** sleep is invoked on Windows
- **THEN** the machine enters sleep

#### Scenario: Sleep is refused

- **WHEN** the operating system refuses the sleep request
- **THEN** the launcher stays open and reports the failure

### Requirement: Empty trash

Empty trash SHALL permanently delete the contents of the platform's trash.
Because the action cannot be undone, it SHALL ask for confirmation before
deleting anything, and the confirmation SHALL name what is about to be lost.

#### Scenario: Confirmation is shown first

- **WHEN** the user invokes empty trash and the trash is not empty
- **THEN** a confirmation is shown stating how many items will be permanently deleted
- **AND** nothing is deleted yet

#### Scenario: Deletion is confirmed

- **WHEN** the user confirms the deletion
- **THEN** the trash is emptied
- **AND** the launcher hides

#### Scenario: Deletion is declined

- **WHEN** the user invokes empty trash and declines the confirmation
- **THEN** nothing is deleted
- **AND** the launcher returns to where it was

#### Scenario: Empty trash on macOS

- **WHEN** empty trash is confirmed on macOS
- **THEN** the user's Trash is emptied

#### Scenario: Empty trash on Windows

- **WHEN** empty trash is confirmed on Windows
- **THEN** the Recycle Bin is emptied

#### Scenario: Trash is already empty

- **WHEN** empty trash is invoked and there is nothing to delete
- **THEN** the command reports success without asking for confirmation

### Requirement: Quit application

Quit application SHALL offer the applications currently running as a list to
choose from, and SHALL ask the chosen one to quit in the ordinary way, so it can
prompt about unsaved work.

#### Scenario: Running applications are listed

- **WHEN** the user invokes quit application
- **THEN** the applications currently running are listed, each with its name and icon

#### Scenario: The list narrows as the user types

- **WHEN** the user types while the running applications are listed
- **THEN** the list narrows to the matching applications

#### Scenario: Chosen application quits

- **WHEN** the user chooses a running application
- **THEN** that application is asked to quit
- **AND** the launcher hides

#### Scenario: Application refuses to quit

- **WHEN** the chosen application does not quit, for example because it is prompting about unsaved work
- **THEN** the launcher does not force it
- **AND** no failure is reported, because the application is behaving correctly

#### Scenario: Dango is not offered

- **WHEN** the running applications are listed
- **THEN** Dango itself does not appear among them

#### Scenario: Application exits before it is chosen

- **WHEN** the chosen application has already exited
- **THEN** the failure is reported
- **AND** the launcher stays open

### Requirement: Open config file

Open config file SHALL be a no-view command, titled `Open Config File`, that
opens the config file the configuration spec names, honouring
`$DANGO_CONFIG_DIR`, with the application the operating system associates with
`.json` files. Dango SHALL NOT choose, bundle, or configure an editor. The
command SHALL be found by the words a user reaches for when looking for
settings, including "config", "settings", "preferences", "keybinds", and
"hotkeys".

When no config file exists, the command SHALL create one containing the
`$schema` key pointing at the schema beside it and the schema version, before
opening it, and the creation SHALL NOT be treated as an external edit to react
to. An existing file SHALL be opened as it is, with no change to its content.

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
- **THEN** a config file containing `"$schema": "./config.schema.json"` and the schema version is created in the config directory
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
