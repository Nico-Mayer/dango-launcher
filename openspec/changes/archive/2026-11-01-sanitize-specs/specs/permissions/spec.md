## MODIFIED Requirements

### Requirement: Dango knows which permissions are missing

On macOS Dango SHALL track two permissions: Accessibility, which pasting,
inserting snippets, reading the selection, AI paste, and window management need,
and Input Monitoring, which keyword expansion and the hyperkey need. Dango SHALL
re-check both each time the launcher is shown, and SHALL keep re-checking while
it runs, so a change is noticed within a few seconds without the launcher being
opened. Checking SHALL NOT show a system prompt.

#### Scenario: Both permissions are granted on macOS

- **WHEN** Accessibility and Input Monitoring are both granted
- **THEN** Dango shows no permission warning anywhere

#### Scenario: Checking on show does not slow activation on macOS

- **WHEN** the launcher is shown
- **THEN** the permission check adds no visible delay to the time from hotkey to painted window

#### Scenario: A grant is noticed without opening the launcher on macOS

- **WHEN** a permission is missing and the user grants it in System Settings without opening the launcher
- **THEN** Dango notices the grant within a few seconds, without the launcher being opened

#### Scenario: Checking never prompts on macOS

- **WHEN** Dango checks the permissions at startup, on show, or while polling
- **THEN** no system permission prompt appears

### Requirement: A missing permission is a warning in root search

Each missing permission SHALL appear as a warning item in root search. On an
empty query the warning items SHALL be listed above every other result. The item
SHALL name the permission and the features it turns off, and SHALL be found by
searching for the permission's name or for "permission".

#### Scenario: Accessibility is missing on macOS

- **WHEN** Accessibility is not granted and the user opens the launcher
- **THEN** the first item reads "Accessibility is off"
- **AND** its subtitle reads "Pasting, snippets, AI paste, and window management don't work without it"

#### Scenario: Input Monitoring is missing on macOS

- **WHEN** Input Monitoring is not granted and the user opens the launcher
- **THEN** the first item reads "Input Monitoring is off"
- **AND** its subtitle reads "Keyword expansion and the hyperkey don't work without it"

#### Scenario: Both are missing on macOS

- **WHEN** neither permission is granted and the user opens the launcher
- **THEN** both warning items are listed above every other result, Accessibility first

#### Scenario: Searching for a warning on macOS

- **WHEN** a permission is missing and the user types "permission"
- **THEN** its warning item is among the results

#### Scenario: The warning goes away after a grant on macOS

- **WHEN** the user grants the permission and opens the launcher again
- **THEN** its warning item is gone

### Requirement: A missing permission is shown in the tray

Each missing permission SHALL appear as an enabled tray item that names the
permission and opens the same System Settings pane as "Grant access". The item
SHALL disappear when the permission is granted, without a restart.

#### Scenario: The tray shows a missing permission on macOS

- **WHEN** Input Monitoring is not granted
- **THEN** the tray menu has an item reading "Grant Input Monitoring"
- **AND** choosing it opens System Settings at Privacy & Security, Input Monitoring

#### Scenario: The tray item goes away after a grant on macOS

- **WHEN** the user grants the permission while Dango runs
- **THEN** within a few seconds, without the launcher being opened, the tray menu no longer has that item
