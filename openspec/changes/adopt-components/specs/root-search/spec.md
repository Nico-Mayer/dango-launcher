# Spec Delta

## ADDED Requirements

### Requirement: Root results can show accessories

A root search result SHALL be able to show up to three accessories on its right,
drawn as a list row's are (see design-system). A permission warning SHALL show
the tag "Needs permission" in the danger role. A command with a global hotkey
SHALL show that hotkey as a key chord, using the platform's names for the keys.

#### Scenario: A permission warning

- **WHEN** Accessibility is off on macOS and the user opens the launcher
- **THEN** the warning's row shows "Needs permission" in the danger role

#### Scenario: A command's hotkey on Windows

- **WHEN** "Clipboard History" has the hotkey "ctrl+alt+v" on Windows and the
  user types "clip"
- **THEN** its row shows "Ctrl+Alt+V" as a key chord

#### Scenario: A command's hotkey on macOS

- **WHEN** "Clipboard History" has the hotkey "ctrl+alt+v" on macOS and the user
  types "clip"
- **THEN** its row shows "⌃⌥V" as a key chord

#### Scenario: A command without a hotkey

- **WHEN** a command has no hotkey
- **THEN** its row shows no key chord
