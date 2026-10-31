## MODIFIED Requirements

### Requirement: Launcher window appearance

The launcher window SHALL be borderless, without a title bar, with a transparent
background, always on top of other windows, and non-resizable by the user.

Its height SHALL follow its content: a list or grid SHALL make the window as
tall as the SearchBar, its rows, and the ActionBar, up to 480 logical pixels,
and scroll beyond that. An empty view, a detail, and a form SHALL use the full
480 logical pixels. A height change SHALL NOT flicker or show the window at an
old and a new size in the same frame. While root search is still receiving
results for a query, the window SHALL grow with them but SHALL NOT shrink, and
"No results" SHALL NOT show, until every provider has answered.

#### Scenario: Window has no system chrome

- **WHEN** the launcher is visible
- **THEN** it shows no title bar, no border, and no window control buttons

#### Scenario: Window floats above other applications

- **WHEN** the launcher is visible and another application is in the foreground
- **THEN** the launcher remains drawn above that application

#### Scenario: Window appears over a fullscreen application on macOS

- **WHEN** the user presses the shortcut while a macOS application occupies a fullscreen space
- **THEN** the launcher is drawn above that application's window, not behind it
- **AND** the system does not switch to another space

#### Scenario: Window appears over a maximized application on Windows

- **WHEN** the user presses the shortcut while a maximized application is in the foreground
- **THEN** the launcher appears over that application

#### Scenario: Few results make a short window

- **WHEN** a query leaves three results
- **THEN** the window is as tall as the SearchBar, three rows, and the
  ActionBar

#### Scenario: A partial answer does not shrink the window

- **WHEN** the user types and a fast provider answers with two results before
  the others answer
- **THEN** the window keeps its height until every provider has answered, and
  then fits the final results

#### Scenario: Many results stop at the maximum

- **WHEN** a query leaves forty results
- **THEN** the window is 480 logical pixels tall and the list scrolls

#### Scenario: Typing changes the height without flicker on Windows

- **WHEN** the user types on Windows and each keystroke changes the number of
  results
- **THEN** the window changes height without flicker or a frame drawn at the
  wrong size

#### Scenario: Typing changes the height without flicker on macOS

- **WHEN** the user types on macOS and each keystroke changes the number of
  results
- **THEN** the window changes height without flicker or a frame drawn at the
  wrong size

### Requirement: Launcher positioning

When shown, the launcher SHALL be positioned on the display that currently holds
the foreground window, centred horizontally and placed above the vertical centre
of that display's work area, so that its maximum height fits inside the work
area. While the launcher is shown, a height change SHALL keep its top edge where
it is and move only its bottom edge.

#### Scenario: Multi-monitor placement

- **WHEN** the foreground window is on a secondary display and the user presses the shortcut
- **THEN** the launcher appears on that secondary display, not the primary one

#### Scenario: Display configuration changes between invocations

- **WHEN** a display is disconnected while the launcher is hidden and the user then presses the shortcut
- **THEN** the launcher appears fully within the bounds of a currently connected display

#### Scenario: Mixed display scaling

- **WHEN** the launcher is shown on a display whose scaling factor differs from the primary display
- **THEN** the launcher renders at the correct physical size for that display with no clipping

#### Scenario: The SearchBar stays put on Windows

- **WHEN** the launcher is shown on Windows and the user types until the window
  shrinks and then grows again
- **THEN** the SearchBar stays at the same place on screen throughout

#### Scenario: The SearchBar stays put on macOS

- **WHEN** the launcher is shown on macOS and the user types until the window
  shrinks and then grows again
- **THEN** the SearchBar stays at the same place on screen throughout
