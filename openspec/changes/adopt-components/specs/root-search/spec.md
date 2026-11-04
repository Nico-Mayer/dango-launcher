# Spec Delta

## ADDED Requirements

### Requirement: A result shows its hotkey

A command or application result whose global hotkey is registered SHALL show
that hotkey on its right as a key chord, using the platform's names for the
keys. A result without a working hotkey SHALL show none.

#### Scenario: A command's hotkey on Windows

- **WHEN** "Clipboard History" has the hotkey "ctrl+alt+v" on Windows and the
  user types "clip"
- **THEN** its row shows "Ctrl+Alt+V" as a key chord

#### Scenario: A command's hotkey on macOS

- **WHEN** "Clipboard History" has the hotkey "ctrl+alt+v" on macOS and the user
  types "clip"
- **THEN** its row shows "⌃⌥V" as a key chord

#### Scenario: An application's hotkey

- **WHEN** an application has a registered hotkey and the user types its name
- **THEN** its row shows that hotkey

#### Scenario: A hotkey that could not be registered

- **WHEN** a command's configured hotkey was refused by the operating system
- **THEN** its row shows no key chord
