## MODIFIED Requirements

### Requirement: Dismissal

The launcher SHALL hide when it loses focus to another application and when the
global shortcut is pressed while it is visible. Both of these are interrupting
hides and SHALL hold the launcher's state as the reset requirement describes.
A click on a transparent part of the launcher window, such as the gap between
the pill and the card, SHALL count as a click outside the launcher.

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

#### Scenario: Clicking the gap dismisses

- **WHEN** the launcher shows the pill and the card and the user clicks in the
  gap between them
- **THEN** the launcher hides
- **AND** its state is held for the grace period

#### Scenario: Shortcut toggles

- **WHEN** the launcher is visible and the user presses the global shortcut
- **THEN** the launcher hides
- **AND** its state is held for the grace period

### Requirement: Launcher window appearance and height

The launcher window SHALL be borderless, without a title bar, with a transparent
background, always on top of other windows, and non-resizable by the user.

On both platforms, the card SHALL be as tall as its content: a list or grid
SHALL make it as tall as an answer card when root search shows one, its rows,
and the ActionBar, so that the pill, the gap, and the card together are at most
480 logical pixels, and the list SHALL scroll beyond that. An empty view, a
detail, and a form SHALL make the launcher the full 480 logical pixels. With
the root view closed, only the pill, or the pill and the "Needs attention"
card, SHALL be shown.

On Windows, the window's height SHALL follow the visible launcher. A height
change SHALL NOT flicker or show the window at an old and a new size in the
same frame. While root search is still receiving results for a query, the
window SHALL grow with them but SHALL NOT shrink until every provider has
answered.

On macOS, the window SHALL stay 480 logical pixels tall for every view, and
the area below the visible launcher SHALL be transparent.

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
- **THEN** the window is as tall as the pill

#### Scenario: The bare pill on macOS

- **WHEN** the launcher opens on macOS with an empty query and no item is
  pinned
- **THEN** the window stays 480 logical pixels tall and everything below the
  pill is transparent

#### Scenario: Few results make a short window on Windows

- **WHEN** a query on Windows leaves three results
- **THEN** the window is as tall as the pill, the gap, three rows, and the
  ActionBar

#### Scenario: Few results keep the full height on macOS

- **WHEN** a query on macOS leaves three results
- **THEN** the window stays 480 logical pixels tall, the card is as tall as
  three rows and the ActionBar, and the area below the card is transparent

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
- **THEN** the window stays 480 logical pixels tall

### Requirement: Reset to root state on hide

Because the window is reused, returning the launcher to its initial state is
an explicit operation, and when it happens SHALL depend on why the launcher
was hidden.

A deliberate hide SHALL reset at once. Deliberate hides are an action that has
finished its work and hidden the launcher, and Escape pressed on an empty root
prompt with the list closed. The next activation SHALL present the launcher as
if it had just started.

An interrupting hide SHALL hold the launcher's state for a grace period.
Interrupting hides are the global shortcut pressed while the launcher is
visible, focus lost to another application, and a click outside the launcher
or on a transparent part of its window.
An activation within the period SHALL show the launcher as it was: the prompt
text, the pushed views, and the selected row intact, with the results for the
current prompt refreshed. When the period ends with the launcher still hidden,
the launcher SHALL reset so that the next activation presents it as if it had
just started, with the first frame already showing the closed root view (see
root-search).

A command still running at an interrupting hide SHALL keep running until the
period ends. Output it produces while hidden SHALL be shown when the launcher
is activated within the period. When the period ends with the launcher still
hidden, the command SHALL be cancelled and its later output dropped. A
deliberate hide SHALL cancel a running command at once.

A command that hides the launcher on its own behalf, to paste, is neither kind
of hide: the launcher SHALL reset its view state and SHALL NOT cancel the
command, whose next view brings the launcher back. Reading a selection SHALL
NOT hide the launcher at all (see `selection-and-paste`).

A failure reported while the launcher is hidden SHALL NOT be kept for the next
activation, whether the launcher was reset or held. The HUD shows it instead
(see `hud`). A background task is the one exception: its result is shown on
the next activation as well (see `background-tasks`).

A background task is not a running command for this requirement: neither kind
of hide, nor the grace period running out, SHALL cancel it.

#### Scenario: State does not leak between invocations

- **WHEN** the user types into the prompt, runs an action that hides the
  launcher, and shows it again
- **THEN** the prompt is empty and no view is pushed

#### Scenario: Shortcut pressed by mistake

- **WHEN** the user types "fire" into the prompt, presses the global shortcut,
  and presses it again within the grace period
- **THEN** the prompt still reads "fire", the results match it, and the row that
  was selected is still selected

#### Scenario: Focus lost on Windows

- **WHEN** a pushed view is on screen on Windows, another application takes the
  foreground, and the user presses the shortcut within the grace period
- **THEN** the same pushed view is on screen with its content intact
- **AND** keyboard input goes to the field that had the cursor before the hide

#### Scenario: Focus lost on macOS

- **WHEN** a pushed view is on screen on macOS, another application takes the
  foreground, and the user presses the shortcut within the grace period
- **THEN** the same pushed view is on screen with its content intact
- **AND** keyboard input goes to the field that had the cursor before the hide

#### Scenario: Held results are refreshed

- **WHEN** the launcher is hidden by an interrupting hide with a prompt typed,
  the data behind one of its results changes while hidden, and the user
  activates within the grace period
- **THEN** the results shown are for the held prompt and reflect the change

#### Scenario: The period runs out

- **WHEN** the launcher is hidden by an interrupting hide and the user activates
  it after the grace period has passed
- **THEN** the prompt is empty, no view is pushed, and the first painted frame
  shows the closed root view

#### Scenario: A running command survives an interrupting hide

- **WHEN** a command is streaming output into a pushed view, the user presses
  the shortcut, and presses it again within the grace period
- **THEN** the view shows everything the command produced so far
- **AND** it keeps updating until the command finishes

#### Scenario: A running command is cancelled when the period runs out

- **WHEN** a command is streaming output into a pushed view, the launcher is
  hidden by an interrupting hide, and the grace period passes
- **THEN** the command is cancelled and its later output is dropped
- **AND** the next activation presents the launcher as if it had just started

#### Scenario: A deliberate hide cancels a running command

- **WHEN** a command is streaming output and the user presses Escape until the
  launcher hides
- **THEN** the command is cancelled before the launcher is gone

#### Scenario: A zero grace period resets at once

- **WHEN** `launcher.reset-after-seconds` is 0 and the launcher is hidden by an
  interrupting hide
- **THEN** the next activation presents the launcher as if it had just started
- **AND** a command that was running is cancelled at the hide

#### Scenario: A command hides the launcher for its own work

- **WHEN** a command hides the launcher to paste and then produces a view
- **THEN** the launcher comes back showing that view
- **AND** the command was not cancelled by the hide

#### Scenario: Reading a selection does not hide the launcher

- **WHEN** a command reads the selection while the launcher is on screen
- **THEN** the launcher stays on screen and keeps its view state

#### Scenario: A failure from a dismissing action is shown afterwards

- **WHEN** an action hides the launcher and then fails
- **THEN** the HUD shows that failure's message at once
- **AND** the next activation shows no failure toast

#### Scenario: The user clears a carried failure

- **WHEN** a failure was reported while the launcher was held by an interrupting
  hide, and the user shows the launcher within the grace period
- **THEN** the held view is back without a failure toast, because the HUD
  already showed the message

#### Scenario: A background task survives the grace period

- **WHEN** an update downloads, the launcher is hidden, and the grace period
  passes
- **THEN** the download keeps going and its view is dropped

