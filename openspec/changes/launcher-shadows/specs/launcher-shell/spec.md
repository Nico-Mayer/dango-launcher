# Spec Delta

## MODIFIED Requirements

### Requirement: Launcher window appearance and height

The launcher window SHALL be borderless, without a title bar, with a transparent
background, always on top of other windows, and non-resizable by the user.
It SHALL hold the visible launcher inside a transparent margin, the same on
every side, wide enough for the shadows design-system describes. The margin
SHALL NOT change where the pill and the card appear on screen. A click on the
margin is a click on a transparent part of the window (see Dismissal).

On both platforms, the card SHALL be as tall as its content: a list or grid
SHALL make it as tall as an answer card when root search shows one, its rows,
and the ActionBar, so that the pill, the gap, and the card together are at most
480 logical pixels, and the list SHALL scroll beyond that. An empty view, a
detail, and a form SHALL make the launcher the full 480 logical pixels. With
the root view closed, only the pill, or the pill and the "Needs attention"
card, SHALL be shown.

On Windows, the window's height SHALL follow the visible launcher plus the
margin above and below it. A height
change SHALL NOT flicker or show the window at an old and a new size in the
same frame. While root search is still receiving results for a query, the
window SHALL grow with them but SHALL NOT shrink until every provider has
answered.

On macOS, the window SHALL stay 480 logical pixels tall plus the margin for
every view, and the area below the visible launcher SHALL be transparent.

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

#### Scenario: The bare pill on Windows

- **WHEN** the launcher opens on Windows with an empty query and no item is
  pinned
- **THEN** the window is as tall as the pill plus the margin above and below

#### Scenario: The bare pill on macOS

- **WHEN** the launcher opens on macOS with an empty query and no item is
  pinned
- **THEN** the window stays 480 logical pixels tall plus the margin, and
  everything below the pill and its shadow is transparent

#### Scenario: Few results make a short window on Windows

- **WHEN** a query on Windows leaves three results
- **THEN** the window is as tall as the pill, the gap, three rows, the
  ActionBar, and the margin above and below

#### Scenario: Few results keep the full height on macOS

- **WHEN** a query on macOS leaves three results
- **THEN** the window stays 480 logical pixels tall plus the margin, the card
  is as tall as three rows and the ActionBar, and the area below the card's
  shadow is transparent

#### Scenario: The margin keeps the launcher in place

- **WHEN** the launcher is shown on either platform
- **THEN** the pill and the card are where they would be without the margin:
  centred horizontally and above the display's vertical centre, as Placement
  describes

#### Scenario: A click on the margin dismisses

- **WHEN** the launcher is visible and the user clicks the margin beside the
  card
- **THEN** the launcher hides, as a click in the gap does

#### Scenario: A partial answer does not shrink the window on Windows

- **WHEN** the user types on Windows and a fast provider answers with two
  results before the others answer
- **THEN** the window keeps its height until every provider has answered, and
  then fits the final results

#### Scenario: Many results stop at the maximum

- **WHEN** a query leaves forty results
- **THEN** the launcher is 480 logical pixels tall and the list scrolls

#### Scenario: Typing changes the height without flicker on Windows

- **WHEN** the user types on Windows and each keystroke changes the number of
  results
- **THEN** the window changes height without flicker or a frame drawn at the
  wrong size

#### Scenario: Typing keeps the height on macOS

- **WHEN** the user types on macOS and each keystroke changes the number of
  results
- **THEN** the window stays 480 logical pixels tall plus the margin
