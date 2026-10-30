## Purpose

Makes the operating system permissions Dango depends on visible: which are
missing, what they turn off, and a direct route to grant them, so no feature
fails silently for want of a permission.

## ADDED Requirements

### Requirement: Dango knows which permissions are missing

On macOS Dango SHALL track two permissions: Accessibility, which pasting,
inserting snippets, reading the selection, AI paste, and window management need,
and Input Monitoring, which keyword expansion and the hyperkey need. Dango SHALL
re-check both each time the launcher is shown, and SHALL re-check at least every
2 seconds while it runs. Checking SHALL NOT show a system prompt.

#### Scenario: Both permissions are granted on macOS

- **WHEN** Accessibility and Input Monitoring are both granted
- **THEN** Dango shows no permission warning anywhere

#### Scenario: Checking on show stays inside the activation budget on macOS

- **WHEN** the launcher is shown
- **THEN** the permission check adds less than 1ms to the time from hotkey to painted window

#### Scenario: A grant is noticed without opening the launcher on macOS

- **WHEN** a permission is missing and the user grants it in System Settings without opening the launcher
- **THEN** Dango notices the grant within 2 seconds

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
- **THEN** the first item reads that Accessibility is off and names pasting, snippets, AI paste, and window management

#### Scenario: Input Monitoring is missing on macOS

- **WHEN** Input Monitoring is not granted and the user opens the launcher
- **THEN** the first item reads that Input Monitoring is off and names keyword expansion and the hyperkey

#### Scenario: Both are missing on macOS

- **WHEN** neither permission is granted and the user opens the launcher
- **THEN** both warning items are listed above every other result, Accessibility first

#### Scenario: Searching for a warning on macOS

- **WHEN** a permission is missing and the user types "permission"
- **THEN** its warning item is among the results

#### Scenario: The warning goes away after a grant on macOS

- **WHEN** the user grants the permission and opens the launcher again
- **THEN** its warning item is gone

### Requirement: The warning offers the route to grant

The warning item's primary action SHALL be "Grant access". It SHALL ask the
system for the permission, which adds Dango to the permission's list in System
Settings, and SHALL open the matching pane of System Settings. It SHALL hide the
launcher. A system prompt SHALL appear only as the result of this action.

#### Scenario: Granting Accessibility on macOS

- **WHEN** the user confirms "Grant access" on the Accessibility warning
- **THEN** the launcher hides
- **AND** System Settings opens at Privacy & Security, Accessibility, with Dango in the list

#### Scenario: Granting Input Monitoring on macOS

- **WHEN** the user confirms "Grant access" on the Input Monitoring warning
- **THEN** the launcher hides
- **AND** System Settings opens at Privacy & Security, Input Monitoring, with Dango in the list

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
- **THEN** within 2 seconds the tray menu no longer has that item

### Requirement: A grant takes effect as macOS allows

An Accessibility grant SHALL take effect without restarting Dango. An Input
Monitoring grant takes effect only after Dango restarts, which macOS offers to
do when the permission is turned on; Dango SHALL start keyword expansion and the
hyperkey on that restart, if they are configured and enabled. A revoke while
Dango runs SHALL bring the warning back.

#### Scenario: Accessibility is granted while Dango runs on macOS

- **WHEN** Dango started without Accessibility and the user grants it
- **THEN** the next paste works without a restart

#### Scenario: Input Monitoring is granted on macOS

- **WHEN** Dango started without Input Monitoring, the hyperkey is configured, the snippets extension is enabled, and the user grants Input Monitoring and lets macOS reopen Dango
- **THEN** the hyperkey works and keywords expand

#### Scenario: A revoke while Dango runs on macOS

- **WHEN** the user revokes a permission while Dango runs
- **THEN** its warning item and tray item come back

### Requirement: Windows shows no permission warnings

On Windows no permission gates these features, so Dango SHALL show no permission
warning in root search or in the tray.

#### Scenario: Opening the launcher on Windows

- **WHEN** the user opens the launcher on Windows
- **THEN** no permission warning item is listed
- **AND** the tray menu has no permission item
