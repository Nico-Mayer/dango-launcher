## MODIFIED Requirements

### Requirement: Launcher positioning

When shown, the launcher SHALL be positioned on the display chosen by
`launcher.display`: the display under the mouse pointer, or the primary
display. It SHALL be centred horizontally and placed above the vertical centre
of that display. On Windows the launcher SHALL be placed within the display's
work area, which leaves out the taskbar, so that its maximum height fits inside
it. On macOS the launcher SHALL be placed against the display's full frame. On
Windows, while the launcher is shown, a height change SHALL keep its top edge
where it is and move only its bottom edge.

#### Scenario: Multi-monitor placement

- **WHEN** `launcher.display` is `"pointer"`, the mouse pointer is on a
  secondary display, and the user presses the shortcut
- **THEN** the launcher appears on that secondary display, not the primary one

#### Scenario: Pinned to the primary display on Windows

- **WHEN** `launcher.display` is `"primary"`, the mouse pointer is on a
  secondary monitor, and the user presses the shortcut on Windows
- **THEN** the launcher appears on the monitor Windows marks as the main
  display

#### Scenario: Pinned to the primary display on macOS

- **WHEN** `launcher.display` is `"primary"`, the mouse pointer is on a
  secondary display, and the user presses the shortcut on macOS
- **THEN** the launcher appears on the display that holds the menu bar

#### Scenario: Display configuration changes between invocations

- **WHEN** a display is disconnected while the launcher is hidden and the user then presses the shortcut
- **THEN** the launcher appears fully within the bounds of a currently connected display

#### Scenario: Mixed display scaling

- **WHEN** the launcher is shown on a display whose scaling factor differs from the primary display
- **THEN** the launcher renders at the correct physical size for that display with no clipping

#### Scenario: Placement on Windows

- **WHEN** the launcher is shown on Windows
- **THEN** it is centred horizontally in the work area of the chosen display
  and placed above its vertical centre

#### Scenario: Placement on macOS

- **WHEN** the launcher is shown on macOS
- **THEN** it is centred horizontally on the full frame of the chosen display
  and placed above its vertical centre

#### Scenario: The SearchBar stays put on Windows

- **WHEN** the launcher is shown on Windows and the user types until the window
  shrinks and then grows again
- **THEN** the SearchBar stays at the same place on screen throughout

#### Scenario: The SearchBar stays put on macOS

- **WHEN** the launcher is shown on macOS and the user types until the number
  of results falls and rises again
- **THEN** the window keeps its size and the SearchBar stays at the same place
  on screen throughout

## ADDED Requirements

### Requirement: The launcher display is a config setting

Dango SHALL read `launcher.display` from the config file. `"pointer"` SHALL
mean the display under the mouse pointer and `"primary"` the primary display.
When the setting is absent, it SHALL mean `"pointer"`. The schema SHALL reject
any other value. A changed value SHALL apply to the next show without a
restart, and the setting SHALL appear in the generated schema and in the
settings reference.

#### Scenario: Setting is absent

- **WHEN** the config file does not mention `launcher.display`
- **THEN** the launcher opens on the display under the mouse pointer

#### Scenario: Setting is changed while running

- **WHEN** the user sets `launcher.display` to `"primary"` and saves the file
  while Dango is running
- **THEN** the next show places the launcher on the primary display, with no
  restart

#### Scenario: An invalid value is rejected

- **WHEN** the user sets `launcher.display` to `"secondary"`
- **THEN** editor validation against the schema reports the value as invalid
- **AND** loading the file reports an error that names the setting

#### Scenario: The setting is documented

- **WHEN** the settings reference is generated
- **THEN** it lists `launcher.display` with its type, default, allowed values,
  and description
