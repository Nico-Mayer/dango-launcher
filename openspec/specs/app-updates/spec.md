# app-updates Specification

## Purpose

Keeps an installed Dango current: it finds newer releases on its own, shows
them where the user already looks, and installs one only when the user
confirms, so updating never means a trip to GitHub.

## Requirements

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
searching for "update" or "Dango". Its primary action SHALL be "Install update"
while nothing is downloading, "Show progress" while the update downloads, and
"Restart now" once the update is ready to restart. Once ready, the item SHALL
read "Dango 0.5.0 is ready to restart".

#### Scenario: The update item on an empty query

- **WHEN** version 0.5.0 is available, Dango 0.4.0 is running, and the user opens the launcher
- **THEN** the first item reads "Dango 0.5.0 is available" with "You have 0.4.0" beneath it
- **AND** its primary action is "Install update"

#### Scenario: The update item while it downloads

- **WHEN** the update downloads and the user opens root search
- **THEN** the update item's primary action is "Show progress", and confirming it
  shows the update's progress view

#### Scenario: The update item once ready

- **WHEN** the update has downloaded and was left with "Install later"
- **THEN** the item reads "Dango 0.5.0 is ready to restart" and its primary
  action is "Restart now"

#### Scenario: A permission warning and an update together on macOS

- **WHEN** Accessibility is off and an update is available
- **THEN** the Accessibility warning is listed first and the update item second

#### Scenario: Searching for the update

- **WHEN** an update is available and the user types "update"
- **THEN** the update item is among the results

#### Scenario: No update, no item

- **WHEN** no update is available
- **THEN** typing "update" lists the "Check for Updates" command and no update item

### Requirement: An update installs only after the user confirms

Dango SHALL download an update only as the result of "Install update", and SHALL
install it only as the result of "Restart now". The update SHALL run as a task
(see background-tasks) with three steps: "Download", "Verify signature", and
"Restart". While it downloads, its progress
view SHALL be titled "Dango" followed by the new version, name the running
version in its subtitle, and show the bytes received and, when known, the total.
Dango SHALL verify the download's signature against the key built into Dango
and SHALL refuse an update whose signature does not match. Once verified, the
update SHALL wait for the user: Enter SHALL run "Restart now", and "Install
later" SHALL leave the progress view with the update kept ready. After
installing, Dango SHALL restart into the new version.

Dismissing the launcher SHALL NOT cancel a download. A failure SHALL be recorded
in `dango.log`, and the update item SHALL stay available. A failure ticker for a
failed download or verification SHALL carry "Retry", which starts the download
again.

#### Scenario: Download progress

- **WHEN** the user confirms "Install update"
- **THEN** the launcher shows the progress view titled "Dango 0.5.0" with the
  download step working and the bytes received so far

#### Scenario: Ready to restart

- **WHEN** the download is verified while its progress view is shown
- **THEN** the "Restart" step needs input, the detail reads "Ready to
  restart", and the ActionBar shows "Restart now" with the Enter key
- **AND** nothing is installed until the user confirms

#### Scenario: Installing on Windows

- **WHEN** the user confirms "Restart now" on Windows
- **THEN** Dango quits and the installer runs with a progress window and needs no input
- **AND** Dango starts again as the new version, with the same config and stored data

#### Scenario: Installing on macOS

- **WHEN** the user confirms "Restart now" on macOS
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

#### Scenario: Retrying a failed download

- **WHEN** a failure ticker for the update carries "Retry" and the user runs it
- **THEN** the download starts again and its progress view is shown

#### Scenario: Dismissed during the download

- **WHEN** the user dismisses the launcher while an update downloads
- **THEN** the download continues and, once verified, the update waits as ready
  to restart

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
in the launcher. If an update is found, the result SHALL offer "Install
update", and the update item SHALL appear in root search.

#### Scenario: Checking

- **WHEN** the user runs "Check for Updates"
- **THEN** the launcher shows "Checking for updates" until the check ends

#### Scenario: An update is found

- **WHEN** the user runs "Check for Updates" and version 0.5.0 is available
- **THEN** the launcher shows "Dango 0.5.0 is available" with "Install update" as the primary action
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
