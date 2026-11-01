## Purpose

The built-in `games` extension finds the games installed through Steam, shows
them in root search and in a grid of cover art, and launches them through Steam.

## ADDED Requirements

### Requirement: Installed Steam games are discoverable in root search

The `games` extension SHALL list every game installed through Steam, from all
of the user's Steam library folders, in root search. A game SHALL be found
whether or not Steam created a shortcut for it. The extension SHALL list
installed games only, not games the user owns but has not installed.

#### Scenario: Game without a shortcut is found

- **WHEN** a game is installed through Steam and has no Start Menu or desktop
  shortcut
- **THEN** typing part of its name in root search shows it

#### Scenario: Game in a second library folder is found

- **WHEN** a game is installed in a Steam library folder other than the one
  inside Steam's own install folder
- **THEN** it appears in root search

#### Scenario: Owned but not installed game

- **WHEN** the user owns a game on Steam that is not installed
- **THEN** it does not appear

#### Scenario: Steam is not installed

- **WHEN** Steam is not installed
- **THEN** root search shows no games and the launcher works as before

### Requirement: Discovery on Windows and macOS

The extension SHALL find Steam and its library folders where Steam installs
them on each platform.

#### Scenario: Steam games on Windows

- **WHEN** Steam is installed on Windows, in its default folder or a folder the
  user chose
- **THEN** its installed games appear in root search

#### Scenario: Steam games on macOS

- **WHEN** Steam is installed on macOS
- **THEN** its installed games appear in root search

### Requirement: Steam tools that are not games are excluded

Entries Steam installs that are not games, such as shared redistributables,
SHALL NOT appear as games.

#### Scenario: Redistributables are hidden

- **WHEN** Steam has installed Steamworks Common Redistributables
- **THEN** it does not appear in root search or in the games grid

### Requirement: Confirming a game launches it through Steam

Confirming a game SHALL ask Steam to start it, and the launcher SHALL hide.
Steam SHALL decide what happens next, including starting itself, showing an
update, or focusing a game that is already running.

#### Scenario: Launch a game

- **WHEN** the user selects a game and presses Enter
- **THEN** Steam is asked to start that game
- **AND** the launcher hides

#### Scenario: Steam is not running

- **WHEN** the user launches a game while Steam is closed
- **THEN** Steam starts and starts the game

#### Scenario: Game was uninstalled

- **WHEN** the user launches a game that was uninstalled after the list was
  last read
- **THEN** the list is read again
- **AND** the game no longer appears

### Requirement: Games show their own icon

Each game SHALL show its icon from Steam's local library cache. When Steam has
no cached icon for a game, the result SHALL show a gamepad icon and stay
launchable. Showing results SHALL NOT wait for icons.

#### Scenario: Cached icon

- **WHEN** Steam has a cached icon for a game
- **THEN** the game's result shows that icon

#### Scenario: No cached icon

- **WHEN** Steam has no cached icon for a game
- **THEN** the result shows a gamepad icon
- **AND** the game can still be launched

### Requirement: The game list stays fresh

The list SHALL be read in the background, never on the activation path. A game installed or uninstalled
through Steam while Dango runs SHALL appear or disappear without a restart.

#### Scenario: Game installed while running

- **WHEN** a game finishes installing through Steam while Dango is running
- **THEN** it becomes searchable without restarting Dango

#### Scenario: Game uninstalled while running

- **WHEN** a game is uninstalled through Steam while Dango is running
- **THEN** it stops appearing in results

#### Scenario: Opening the launcher does not wait for Steam

- **WHEN** the user opens the launcher while the game list is being read
- **THEN** the launcher opens without waiting

### Requirement: Search Games shows a grid of cover art

The extension SHALL provide a "Search Games" command that opens a grid of the
installed games. Each tile SHALL show the game's cover art from Steam's local
library cache, in portrait shape, with the game's name. Typing SHALL filter
the grid by name, and Enter SHALL launch the selected game as root search does.

#### Scenario: Open the grid

- **WHEN** the user runs "Search Games"
- **THEN** a grid of every installed game is shown, each tile with its cover
  art and name

#### Scenario: Filter the grid

- **WHEN** the user types part of a game's name in the grid
- **THEN** only matching games remain

#### Scenario: Launch from the grid

- **WHEN** the user selects a tile and presses Enter
- **THEN** Steam is asked to start that game
- **AND** the launcher hides

#### Scenario: No cached cover art

- **WHEN** Steam has no cached cover art for a game
- **THEN** its tile shows a gamepad icon and its name

#### Scenario: No games installed

- **WHEN** the user runs "Search Games" and no Steam game is installed, or
  Steam is not installed
- **THEN** the grid shows an empty state that names the missing games and
  says how they get there
