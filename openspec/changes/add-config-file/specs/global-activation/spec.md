## MODIFIED Requirements

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
