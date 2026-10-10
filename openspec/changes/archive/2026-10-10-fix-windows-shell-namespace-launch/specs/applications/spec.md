## MODIFIED Requirements

### Requirement: Discovery on Windows

On Windows the extension SHALL enumerate the applications known to the shell so
that packaged applications from the Microsoft Store and conventional desktop
applications both appear, with the identifier needed to launch each.

#### Scenario: Desktop application is found

- **WHEN** a conventional desktop application is installed
- **THEN** it appears in the index

#### Scenario: Store application is found

- **WHEN** a packaged application from the Microsoft Store is installed
- **THEN** it appears in the index

#### Scenario: Shell namespace application is found and launches

- **WHEN** the shell lists an application whose target is a shell namespace
  identifier rather than a file, such as File Explorer
- **THEN** it appears in the index with its name alone, no path beside it
- **AND** it offers no reveal or copy path action
- **AND** confirming it opens the application
- **AND** it stays in the index afterwards

#### Scenario: Uninstaller entries are excluded

- **WHEN** the shell exposes uninstall or maintenance entries alongside real applications
- **THEN** they do not appear as launchable results

### Requirement: The index stays fresh

The index SHALL be refreshed so that applications installed or removed while
Dango is running become correct without a restart. The platform's own list of
applications SHALL be the only authority on whether an application is gone: a
failed launch SHALL NOT remove an entry unless a fresh enumeration no longer
lists it.

#### Scenario: Newly installed application appears

- **WHEN** an application is installed while Dango is running
- **THEN** it becomes searchable without restarting Dango

#### Scenario: Removed application disappears

- **WHEN** an indexed application is uninstalled
- **THEN** it stops appearing in results

#### Scenario: Stale entry fails to launch

- **WHEN** the user launches an indexed application that the platform no
  longer lists
- **THEN** the index is refreshed from the platform
- **AND** the user is told the application is no longer installed and has been
  removed from results
- **AND** the entry is gone from the index

#### Scenario: Launch fails for an installed application

- **WHEN** a launch fails and a fresh enumeration still lists the application
- **THEN** the user is told the application could not be opened
- **AND** the entry stays in the index

#### Scenario: Confirming a result the index no longer holds

- **WHEN** the user confirms a result whose entry is missing from the index,
  because a refresh replaced the index after the results were shown
- **THEN** the index is refreshed from the platform and the lookup is retried
- **AND** the application opens if the refreshed index lists it

#### Scenario: Enumeration fails during a refresh

- **WHEN** a refresh cannot read the platform's list of applications
- **THEN** the previous index is kept
- **AND** no entry is reported as gone because of that refresh
