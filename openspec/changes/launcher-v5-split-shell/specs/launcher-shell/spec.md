## MODIFIED Requirements

### Requirement: Dismissal

The launcher SHALL hide when it loses focus to another application and when the
global shortcut is pressed while it is visible. Both of these are interrupting
hides and SHALL hold the launcher's state as the reset requirement describes.

At the root prompt, Escape SHALL act in stages. With text in the prompt,
Escape SHALL clear the prompt and leave the launcher open. With an empty
prompt and the empty-query list open (see root-search), Escape SHALL close
that list and leave the launcher open. With an empty prompt and the list
closed, Escape SHALL hide the launcher. This is a deliberate hide. In a pushed
view, Escape SHALL pop that view, whether or not the prompt holds text, as the
view stack in view-protocol describes.

#### Scenario: Escape clears a non-empty prompt

- **WHEN** the launcher shows the root prompt with text in it and the user presses Escape
- **THEN** the prompt is cleared
- **AND** the launcher stays visible, showing the closed root view

#### Scenario: Escape closes the empty-query list

- **WHEN** the user has opened the empty-query list with Arrow Down and
  presses Escape
- **THEN** the list closes and the launcher shows the closed root view
- **AND** the launcher stays visible

#### Scenario: Escape on an empty prompt dismisses

- **WHEN** the launcher shows the root prompt, empty, with the list closed,
  and the user presses Escape
- **THEN** the launcher hides
- **AND** focus returns to the application that was in the foreground beforehand

#### Scenario: Escape with only pinned items shown dismisses

- **WHEN** the launcher shows the closed root view with the "Needs attention"
  card and the user presses Escape
- **THEN** the launcher hides

#### Scenario: Escape in a pushed view pops it

- **WHEN** a pushed view is on screen with text in the prompt and the user
  presses Escape
- **THEN** the view is popped and the launcher stays visible

#### Scenario: Clicking away dismisses

- **WHEN** the launcher is visible and the user clicks another application
- **THEN** the launcher hides
- **AND** its state is held for the grace period

#### Scenario: Shortcut toggles

- **WHEN** the launcher is visible and the user presses the global shortcut
- **THEN** the launcher hides
- **AND** its state is held for the grace period

### Requirement: Launcher window appearance and height

The launcher window SHALL be borderless, without a title bar, with a transparent
background, always on top of other windows, and non-resizable by the user.
The window SHALL be larger than the visible pill and card by a transparent
margin on every side, wide enough for their shadows.

On Windows, its height SHALL follow its content: with the root view closed,
the window SHALL be as tall as the pill, or the pill, the gap, and the
"Needs attention" card when an item is pinned. A list or grid SHALL make the
window as tall as the pill, the gap, an answer card when root search shows
one, its rows, and the ActionBar, up to 480 logical pixels for the visible
launcher, and scroll beyond that. An empty view, a detail, and a form SHALL
use the full 480 logical pixels. A height change SHALL NOT flicker or show the
window at an old and a new size in the same frame. While root search is still
receiving results for a query, the window SHALL grow with them but SHALL NOT
shrink until every provider has answered.

On macOS, the window SHALL stay 480 logical pixels tall for every view, and a
list or grid SHALL scroll beyond that. The area below the visible launcher
SHALL be transparent.

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

#### Scenario: The bare pill makes a short window on Windows

- **WHEN** the launcher opens on Windows with an empty query and no item is
  pinned
- **THEN** the window is as tall as the pill and its margin

#### Scenario: The bare pill on macOS

- **WHEN** the launcher opens on macOS with an empty query and no item is
  pinned
- **THEN** the window stays 480 logical pixels tall and everything below the
  pill is transparent

#### Scenario: Few results make a short window on Windows

- **WHEN** a query on Windows leaves three results
- **THEN** the window is as tall as the pill, the gap, three rows, the
  ActionBar, and its margin

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
- **THEN** the visible launcher is 480 logical pixels tall and the list scrolls

#### Scenario: Typing changes the height without flicker on Windows

- **WHEN** the user types on Windows and each keystroke changes the number of
  results
- **THEN** the window changes height without flicker or a frame drawn at the
  wrong size

#### Scenario: Typing keeps the height on macOS

- **WHEN** the user types on macOS and each keystroke changes the number of
  results
- **THEN** the window stays 480 logical pixels tall

## ADDED Requirements

### Requirement: A click on a transparent part hides the launcher

A click on a transparent part of the launcher window, such as the gap between
the pill and the card, the shadow margin, or on macOS the area below the
visible launcher, SHALL hide the launcher as clicking away does. It is an
interrupting hide. It SHALL NOT act on the pill, the card, or their contents.

#### Scenario: Clicking the gap on Windows

- **WHEN** the launcher shows the pill and the card on Windows and the user
  clicks in the gap between them
- **THEN** the launcher hides and its state is held for the grace period

#### Scenario: Clicking below the pill on macOS

- **WHEN** the launcher shows the bare pill on macOS and the user clicks where
  the card would be
- **THEN** the launcher hides and its state is held for the grace period
