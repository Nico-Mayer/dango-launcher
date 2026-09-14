## ADDED Requirements

### Requirement: An application is addressable by its shown name

An application SHALL be addressable from the configuration file by the name
Dango shows for it in root search, compared ignoring case. An entry MAY carry a
`name` object with `macos` and `windows` values for an application whose shown
name differs between platforms; without one, the entry's key is the name on
both. When more than one indexed application carries the name, the first in
the index's name order SHALL be used and the ambiguity SHALL be surfaced.

Launching an application by hotkey SHALL have the same effect as selecting it
in root search and confirming: the same launch, the same frecency credit, and
the same handling of an entry that turns out to be gone.

#### Scenario: Matching the shown name on macOS

- **WHEN** an entry is keyed `safari` and the index shows an application named
  "Safari"
- **THEN** the entry addresses that application

#### Scenario: Matching the shown name on Windows

- **WHEN** an entry is keyed `notepad` and the shell lists an application
  named "Notepad"
- **THEN** the entry addresses that application

#### Scenario: A name that differs per platform

- **WHEN** an entry carries `name: { macos: "TextEdit", windows: "Notepad" }`
- **THEN** macOS addresses "TextEdit" and Windows addresses "Notepad"

#### Scenario: A name given for the other platform only

- **WHEN** an entry's `name` object has a value only for the other platform
- **THEN** the entry is ignored on this platform and no chord is registered for
  it

#### Scenario: Two applications with one name

- **WHEN** two indexed applications share the bound name
- **THEN** the first in name order is launched
- **AND** the ambiguity is surfaced through the tray status line and the log

#### Scenario: A launch from a hotkey counts as use

- **WHEN** an application is launched from its hotkey
- **THEN** its frecency is credited as if it had been launched from root search

#### Scenario: A bound application has been removed from disk

- **WHEN** the bound application's index entry no longer exists on disk and the
  user presses its chord
- **THEN** the failure is surfaced
- **AND** the entry is removed from the index
