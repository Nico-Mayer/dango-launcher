## MODIFIED Requirements

### Requirement: Discovery on Windows

On Windows the extension SHALL enumerate the applications known to the shell so
that packaged applications from the Microsoft Store and conventional desktop
applications both appear, with the identifier needed to launch each. Shortcuts
that start a Steam game SHALL be left out, because the `games` extension lists
those games.

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

#### Scenario: Steam game shortcuts are excluded

- **WHEN** the shell lists a shortcut that starts a Steam game
- **THEN** it does not appear in the index
- **AND** the game appears once in root search, from the `games` extension

#### Scenario: Steam itself is still found

- **WHEN** Steam is installed
- **THEN** Steam appears in the index as an application
