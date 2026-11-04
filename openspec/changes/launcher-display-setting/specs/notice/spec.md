## RENAMED Requirements

- FROM: `### Requirement: The notice sits at the bottom of the active screen`
- TO: `### Requirement: The notice sits at the bottom of the launcher's display`

## MODIFIED Requirements

### Requirement: The notice sits at the bottom of the launcher's display

The notice SHALL show centred horizontally near the bottom of the work area of
the display the launcher would open on, as chosen by `launcher.display`, above
the taskbar or the Dock. It SHALL show over every other window, including
fullscreen applications.

#### Scenario: Placement on Windows

- **WHEN** the notice shows on Windows with the mouse pointer on a second
  monitor and `launcher.display` is `"pointer"`
- **THEN** it is centred near the bottom of that monitor's work area, above the
  taskbar

#### Scenario: Placement on macOS

- **WHEN** the notice shows on macOS with the mouse pointer on a second display
  and `launcher.display` is `"pointer"`
- **THEN** it is centred near the bottom of that display's visible frame, above
  the Dock

#### Scenario: Pinned to the primary display on Windows

- **WHEN** the notice shows on Windows with the mouse pointer on a second
  monitor and `launcher.display` is `"primary"`
- **THEN** it is centred near the bottom of the main display's work area, above
  the taskbar

#### Scenario: Pinned to the primary display on macOS

- **WHEN** the notice shows on macOS with the mouse pointer on a second display
  and `launcher.display` is `"primary"`
- **THEN** it is centred near the bottom of the visible frame of the main
  display set in System Settings > Displays, above the Dock

#### Scenario: Over a fullscreen application on Windows

- **WHEN** a fullscreen application is in front on Windows and the notice shows
- **THEN** the notice is visible over it

#### Scenario: Over a fullscreen application on macOS

- **WHEN** a fullscreen application is in front in its own Space on macOS and
  the notice shows
- **THEN** the notice is visible over it, in that Space
