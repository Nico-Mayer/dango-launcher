# applications Specification

## Purpose

The built-in extension that discovers installed applications on the machine,
presents them in root search with icons, and launches them. It is also the first
real consumer of the extension contract.

## Requirements

### Requirement: Applications are discoverable in root search

The `applications` extension SHALL contribute installed applications to root
search so that typing part of an application's name and pressing Enter launches
it.

#### Scenario: Launch by exact name

- **WHEN** the user types an installed application's full name and presses Enter
- **THEN** that application is launched or brought to the foreground
- **AND** the launcher hides

#### Scenario: Launch by partial name

- **WHEN** the user types a prefix of an installed application's name
- **THEN** that application appears in the results

#### Scenario: Already running application

- **WHEN** the user launches an application that is already running
- **THEN** its existing window is brought to the foreground rather than a second instance being started

### Requirement: Discovery on macOS

On macOS the extension SHALL discover application bundles in the system
applications directory, the user's applications directory, the system's own
applications directory, and their immediate subdirectories.

#### Scenario: User-installed application is found

- **WHEN** an application bundle is present in the user's applications directory
- **THEN** it appears in the index

#### Scenario: Application in a subfolder is found

- **WHEN** an application bundle sits inside a category subfolder of an applications directory
- **THEN** it appears in the index

#### Scenario: Display name honours localisation

- **WHEN** an application's bundle carries a localised display name
- **THEN** that localised name is used

#### Scenario: Display name follows the system

- **WHEN** an application declares a display name that is not a localisation of
  its bundle filename, which macOS itself disregards
- **THEN** the name shown matches the one the system shows, so a launcher result
  never disagrees with Finder

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

#### Scenario: Uninstaller entries are excluded

- **WHEN** the shell exposes uninstall or maintenance entries alongside real applications
- **THEN** they do not appear as launchable results

### Requirement: Icons are shown and cached

Each result SHALL display the application's own icon. Icons SHALL be extracted
once and cached, and SHALL NOT be extracted on the search path.

#### Scenario: Icon is displayed

- **WHEN** an application appears in results
- **THEN** its own icon is shown beside its name

#### Scenario: Icon extraction does not slow search

- **WHEN** a query returns applications whose icons have not been cached yet
- **THEN** results still appear within the search budget
- **AND** icons fill in as they become available

#### Scenario: Icon cannot be extracted

- **WHEN** an application's icon cannot be read
- **THEN** a placeholder icon is shown and the result is still launchable

### Requirement: The index is built in the background

Indexing SHALL run off the activation path. The launcher SHALL be usable while
the first index is being built.

#### Scenario: First run while indexing

- **WHEN** the user opens the launcher during the initial index build
- **THEN** the launcher opens within its activation budget
- **AND** applications appear as they are indexed

#### Scenario: Index persists across restarts

- **WHEN** Dango restarts after a completed index
- **THEN** applications are searchable immediately without waiting for a rebuild

### Requirement: The index stays fresh

The index SHALL be refreshed so that applications installed or removed while
Dango is running become correct without a restart.

#### Scenario: Newly installed application appears

- **WHEN** an application is installed while Dango is running
- **THEN** it becomes searchable without restarting Dango

#### Scenario: Removed application disappears

- **WHEN** an indexed application is uninstalled
- **THEN** it stops appearing in results

#### Scenario: Stale entry fails to launch

- **WHEN** the user launches an indexed application that no longer exists on disk
- **THEN** the failure is shown
- **AND** the entry is removed from the index

### Requirement: Actions beyond launching

An application result SHALL offer secondary actions in its action panel: at
minimum revealing it in the platform's file manager and copying its path.

#### Scenario: Reveal in file manager

- **WHEN** the user chooses the reveal action on an application result
- **THEN** the platform's file manager opens with that application selected

#### Scenario: Copy path

- **WHEN** the user chooses the copy path action
- **THEN** the application's location is placed on the clipboard

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
