## MODIFIED Requirements

### Requirement: Casing is sentence case, except for names

Text SHALL use sentence case: the first word capitalised, the rest lower case
except for proper nouns and product names. This applies to action labels, form
labels, placeholders, empty states, toasts, ActionBar labels, section headings,
tags, tray items, and messages.

Command titles in root search SHALL be Title Case, because they are shown in
one list with application names and are searched as names. Extension names
SHALL be Title Case for the same reason. Command titles and extension names
are the only Title Case text in the interface.

Platform components keep their own names: Finder, File Explorer, Trash,
Recycle Bin, Accessibility, Cmd, Ctrl, Option, Alt, Enter, Escape.

#### Scenario: Action labels are sentence case

- **WHEN** an action panel is open for any result
- **THEN** every action label reads in sentence case, such as "Show in Finder"
  or "Copy path"

#### Scenario: Command titles are Title Case

- **WHEN** root search lists a built-in command
- **THEN** its title is Title Case, such as "Empty Trash" or "Clipboard History"

#### Scenario: A message is sentence case

- **WHEN** a failure, empty state, toast, or tray notice is shown
- **THEN** it starts with a capital letter and is otherwise lower case except
  for names

### Requirement: Tray notices say what is wrong and where to look

A tray notice about a startup problem SHALL name the thing that failed and,
when the detail is in the log, SHALL name the log file. A notice about the
launcher hotkey SHALL name the chord and say what to change. The config status
line SHALL say whether the config file loaded.

#### Scenario: Hotkeys conflict

- **WHEN** one or more command hotkeys could not be registered at startup
- **THEN** a disabled tray item reads that some hotkeys are already in use and
  names `dango.log`

#### Scenario: The launcher hotkey is taken on macOS

- **WHEN** Option+Space is already registered by another application
- **THEN** a tray item says Option+Space is in use by another app and points to
  the launcher hotkey setting in the config file

#### Scenario: The launcher hotkey is taken on Windows

- **WHEN** Alt+Space is already registered by another application
- **THEN** a tray item says Alt+Space is in use by another app and points to
  the launcher hotkey setting in the config file

#### Scenario: The config file has an error

- **WHEN** the config file did not parse
- **THEN** the tray status line reads "Config file has an error" and names
  `dango.log`

#### Scenario: The config file loaded

- **WHEN** the config file parsed
- **THEN** the tray status line reads "Config file loaded"

#### Scenario: Tray menu items

- **WHEN** the tray menu is open and no permission is missing, which is always
  the case on Windows
- **THEN** the actionable items read "Show or hide Dango" and "Quit Dango"

#### Scenario: Tray menu items with a missing permission on macOS

- **WHEN** the tray menu is open on macOS and a permission is missing
- **THEN** the actionable items read "Show or hide Dango" and "Quit Dango"
- **AND** when Accessibility is missing, an item reads "Grant Accessibility"
- **AND** when Input Monitoring is missing, an item reads "Grant Input
  Monitoring"

