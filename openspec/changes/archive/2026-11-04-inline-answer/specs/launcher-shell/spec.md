# Spec Delta

## MODIFIED Requirements

### Requirement: Launcher window appearance and height

The launcher window SHALL be borderless, without a title bar, with a transparent
background, always on top of other windows, and non-resizable by the user.

On Windows, its height SHALL follow its content: a list or grid SHALL make the
window as tall as the SearchBar, an answer card when root search shows one,
its rows, and the ActionBar, up to 480 logical
pixels, and scroll beyond that. An empty view, a detail, and a form SHALL use
the full 480 logical pixels. A height change SHALL NOT flicker or show the
window at an old and a new size in the same frame. While root search is still
receiving results for a query, the window SHALL grow with them but SHALL NOT
shrink until every provider has answered.

On macOS, the window SHALL stay 480 logical pixels tall for every view, and a
list or grid SHALL scroll beyond that.

On both platforms, "No results" SHALL NOT show while root search is still
receiving results for a query, until every provider has answered.

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

#### Scenario: Few results make a short window on Windows

- **WHEN** a query on Windows leaves three results
- **THEN** the window is as tall as the SearchBar, three rows, and the
  ActionBar

#### Scenario: Few results keep the full height on macOS

- **WHEN** a query on macOS leaves three results
- **THEN** the window stays 480 logical pixels tall

#### Scenario: A partial answer does not shrink the window on Windows

- **WHEN** the user types on Windows and a fast provider answers with two
  results before the others answer
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

#### Scenario: Typing keeps the height on macOS

- **WHEN** the user types on macOS and each keystroke changes the number of
  results
- **THEN** the window stays 480 logical pixels tall
