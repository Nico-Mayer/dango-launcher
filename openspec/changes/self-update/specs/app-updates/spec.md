## Purpose

Keeps an installed Dango current: it finds newer releases on its own, shows
them where the user already looks, and installs one only when the user
confirms, so updating never means a trip to GitHub.

## ADDED Requirements

### Requirement: Dango checks for updates in the background

A release build of Dango SHALL check for a newer release once shortly after
startup and then every 6 hours while it runs. A check SHALL NOT delay startup,
the hotkey, or search, and SHALL NOT show anything when no update exists or the
check fails. A development build SHALL never check.

Only a published release with a higher version than the running one counts as
an update. Drafts and pre-releases do not.

#### Scenario: An update is found after startup on Windows

- **WHEN** a release build starts on Windows and a newer release is published
- **THEN** the update item appears in root search without the user doing anything

#### Scenario: An update is found after startup on macOS

- **WHEN** a release build starts on macOS and a newer release is published
- **THEN** the update item appears in root search without the user doing anything

#### Scenario: A release published while Dango runs

- **WHEN** Dango has been running and a newer release is published
- **THEN** the update item appears within 6 hours without a restart

#### Scenario: The check does not slow activation

- **WHEN** the user summons the launcher while a check is in progress
- **THEN** the launcher appears and searches without visible lag

#### Scenario: The check fails quietly

- **WHEN** the automatic check fails, for example because the machine is offline
- **THEN** nothing is shown to the user
- **AND** `dango.log` records the failure
- **AND** the next scheduled check runs as usual

#### Scenario: Running the newest version

- **WHEN** the running version is the newest published release
- **THEN** no update item is shown

#### Scenario: Development builds never check

- **WHEN** a development build runs
- **THEN** it makes no update request at any time

### Requirement: An available update is an item at the top of root search

When an update is available, root search SHALL show one item naming the new
version and the running version. On an empty query it SHALL be listed above
every other result except the permission warnings. It SHALL be found by
searching for "update" or "Dango". Its primary action SHALL be "Install and
restart".

#### Scenario: The update item on an empty query

- **WHEN** version 0.5.0 is available, Dango 0.4.0 is running, and the user opens the launcher
- **THEN** the first item reads "Dango 0.5.0 is available" with "You have 0.4.0" beneath it
- **AND** its primary action is "Install and restart"

#### Scenario: A permission warning and an update together on macOS

- **WHEN** Accessibility is off and an update is available
- **THEN** the Accessibility warning is listed first and the update item second

#### Scenario: Searching for the update

- **WHEN** an update is available and the user types "update"
- **THEN** the update item is among the results

#### Scenario: No update, no item

- **WHEN** no update is available
- **THEN** typing "update" lists the "Check for Updates" command and no update item

### Requirement: An available update is shown in the tray

When an update is available, the tray menu SHALL show an enabled item reading
"Install Dango" followed by the new version. Choosing it SHALL show the
launcher and start the same install as "Install and restart". The item SHALL be
absent when no update is available.

#### Scenario: Tray item on Windows

- **WHEN** version 0.5.0 is available and the user right-clicks the tray icon on Windows
- **THEN** the menu has an item reading "Install Dango 0.5.0"

#### Scenario: Tray item on macOS

- **WHEN** version 0.5.0 is available and the user clicks the menu bar icon on macOS
- **THEN** the menu has an item reading "Install Dango 0.5.0"

#### Scenario: Installing from the tray

- **WHEN** the user chooses "Install Dango 0.5.0" in the tray
- **THEN** the launcher appears showing the download, as after "Install and restart"

#### Scenario: No update in the tray

- **WHEN** no update is available
- **THEN** the tray menu has no install item

### Requirement: An update installs only after the user confirms

Dango SHALL download and install an update only as the result of "Install and
restart" or the tray item. While it downloads, the launcher SHALL show a view
naming the version and the download's progress. Dango SHALL verify the
download's signature against the key built into Dango and SHALL refuse an
update whose signature does not match. After installing it SHALL restart into
the new version.

Dismissing the launcher SHALL NOT cancel a download. A failure while the
launcher is hidden SHALL be recorded in `dango.log`, and the update item SHALL
stay available.

#### Scenario: Download progress

- **WHEN** the user confirms "Install and restart"
- **THEN** the launcher shows "Downloading Dango 0.5.0" with the progress so far

#### Scenario: Installing on Windows

- **WHEN** the download of an update completes on Windows
- **THEN** Dango quits and the installer runs with a progress window and needs no input
- **AND** Dango starts again as the new version, with the same config and stored data

#### Scenario: Installing on macOS

- **WHEN** the download of an update completes on macOS
- **THEN** Dango quits and starts again as the new version, with the same config and stored data

#### Scenario: The download does not match its signature

- **WHEN** the downloaded update fails signature verification
- **THEN** nothing is installed and the running version keeps running
- **AND** the launcher shows "Couldn't verify the update, so it wasn't installed. Try again later."

#### Scenario: The download fails

- **WHEN** the download fails or is interrupted while the launcher is shown
- **THEN** nothing is installed and the running version keeps running
- **AND** the launcher shows "Couldn't download the update. Check your connection and try again."
- **AND** the update item stays available

#### Scenario: Dismissed during the download

- **WHEN** the user dismisses the launcher while an update downloads
- **THEN** the download continues and the install proceeds when it completes

#### Scenario: Nothing installs without confirmation

- **WHEN** an update is available and the user never confirms it
- **THEN** Dango keeps running the current version indefinitely

#### Scenario: Autostart survives an update on Windows

- **WHEN** autostart is on and an update is installed on Windows
- **THEN** the updated Dango starts at the next sign-in

#### Scenario: Autostart survives an update on macOS

- **WHEN** autostart is on and an update is installed on macOS
- **THEN** the updated Dango starts at the next login

### Requirement: The user can check for updates on demand

A built-in "Check for Updates" command SHALL check at once and show the result
in the launcher. If an update is found, the result SHALL offer "Install and
restart", and the update item SHALL appear in root search and the tray.

#### Scenario: Checking

- **WHEN** the user runs "Check for Updates"
- **THEN** the launcher shows "Checking for updates" until the check ends

#### Scenario: An update is found

- **WHEN** the user runs "Check for Updates" and version 0.5.0 is available
- **THEN** the launcher shows "Dango 0.5.0 is available" with "Install and restart" as the primary action
- **AND** the update item is at the top of root search the next time the launcher opens

#### Scenario: Already up to date

- **WHEN** the user runs "Check for Updates" and the running version is the newest
- **THEN** the launcher shows "You have the latest version" and the running version

#### Scenario: The check fails

- **WHEN** the user runs "Check for Updates" and the check fails
- **THEN** the launcher shows "Couldn't check for updates. Check your connection and try again."

#### Scenario: Development build

- **WHEN** the user runs "Check for Updates" in a development build
- **THEN** the launcher shows "Updates are off in development builds."
- **AND** no update request is made
