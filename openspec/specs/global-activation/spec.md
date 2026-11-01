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

### Requirement: A hotkey needs a modifier unless it is a function key

Every hotkey in the configuration file, the launcher's, a command's, and an
application's, SHALL name at least one modifier, unless its key is a function
key from F1 to F20, which MAY stand alone. Function keys F1 to F20 SHALL be
accepted as keys with or without modifiers, on Windows and macOS alike. Any
other key without a modifier, and a modifier alone (such as the Windows key or
the Command key), SHALL be refused and never registered with the operating
system.

On macOS the hyperkey is built on F18, so while the hyperkey is on, a hotkey
that uses F18 SHALL NOT be expected to fire. Dango does not refuse it; the
settings reference SHALL state the clash.

When the launcher hotkey is refused, Dango SHALL summon the launcher with the
platform default instead, write the reason to the log, and show a tray notice
that the configured hotkey is invalid and the default is in use.

#### Scenario: A bare letter is refused for the launcher

- **WHEN** the configuration file sets the launcher hotkey to `a`
- **THEN** the `a` key is not registered and still types `a` in other applications
- **AND** the platform default summons the launcher
- **AND** the log says why the hotkey was refused and the tray shows a notice

#### Scenario: A bare Space is refused

- **WHEN** the configuration file sets the launcher hotkey to `space`
- **THEN** Space is not registered and keeps working in other applications
- **AND** the platform default summons the launcher

#### Scenario: A lone function key summons the launcher

- **WHEN** the configuration file sets the launcher hotkey to `f13`
- **THEN** pressing F13 alone summons the launcher

#### Scenario: A function key with modifiers

- **WHEN** the configuration file sets the launcher hotkey to `ctrl+f20`
- **THEN** pressing Ctrl+F20 summons the launcher

#### Scenario: A function key beyond F20 is refused

- **WHEN** the configuration file sets the launcher hotkey to `f21`
- **THEN** the hotkey is refused as an unknown key
- **AND** the platform default summons the launcher

#### Scenario: A lone modifier is refused

- **WHEN** the configuration file sets the launcher hotkey to `win` or `cmd`
- **THEN** the hotkey is refused because it has no key
- **AND** the platform default summons the launcher

#### Scenario: F18 with the hyperkey on macOS

- **WHEN** on macOS the hyperkey is on and a hotkey is set to `f18`
- **THEN** pressing F18 does not trigger that hotkey
- **AND** the settings reference says the hyperkey uses F18 on macOS

#### Scenario: F18 with the hyperkey on Windows

- **WHEN** on Windows the hyperkey is on and a hotkey is set to `f18`
- **THEN** pressing F18 triggers that hotkey
