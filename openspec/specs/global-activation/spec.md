# global-activation Specification

## Purpose

The system-wide keyboard shortcut that summons the launcher from any
application, including how registration failures are surfaced when another
application already owns the combination.

## Requirements

### Requirement: Global summoning shortcut

Dango SHALL register one system-wide keyboard shortcut that toggles the launcher
regardless of which application has focus. The shortcut SHALL be read from the
configuration file; when the file does not set it, the default SHALL be
Option+Space on macOS and Alt+Space on Windows.

#### Scenario: Summon from another application

- **WHEN** a text editor has focus and the user presses the shortcut
- **THEN** the launcher appears and receives keyboard input
- **AND** the keystroke is not delivered to the text editor

#### Scenario: Summon while no window has focus

- **WHEN** the user presses the shortcut with the desktop focused and no application window active
- **THEN** the launcher appears and receives keyboard input

#### Scenario: Shortcut works after the session has been idle

- **WHEN** the machine wakes from sleep and the user presses the shortcut
- **THEN** the launcher appears

#### Scenario: The configured shortcut replaces the default

- **WHEN** the configuration file sets the launcher hotkey to a different chord
- **THEN** that chord summons the launcher
- **AND** the platform default no longer does

#### Scenario: An unset shortcut uses the platform default

- **WHEN** the configuration file does not set the launcher hotkey
- **THEN** Option+Space on macOS or Alt+Space on Windows summons the launcher

### Requirement: Registration failure is visible

If the shortcut cannot be registered because another application already owns
it, Dango SHALL continue running and SHALL make the failure discoverable rather
than failing silently or exiting.

#### Scenario: Combination already taken

- **WHEN** another application holds the default combination at Dango startup
- **THEN** Dango starts and its tray icon appears
- **AND** the tray menu indicates that the shortcut is unavailable
- **AND** the launcher can still be opened from the tray menu

### Requirement: Shortcut is released on exit

The global shortcut SHALL be unregistered when Dango exits, including when it is
quit from the tray menu.

#### Scenario: Another application can claim the shortcut afterwards

- **WHEN** Dango has exited
- **THEN** another application can register the same combination successfully
