## Purpose

The system-wide keyboard shortcut that summons the launcher from any
application, including how registration failures are surfaced when another
application already owns the combination.

## ADDED Requirements

### Requirement: Global summoning shortcut

Dango SHALL register one system-wide keyboard shortcut that toggles the launcher
regardless of which application has focus. The default SHALL be
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
