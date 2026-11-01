## MODIFIED Requirements

### Requirement: Warm launcher window

The launcher window SHALL be created hidden during application startup and SHALL
persist for the lifetime of the process. Showing and hiding the launcher SHALL
NOT create or destroy the window or build its interface again.

#### Scenario: Window exists before first use

- **WHEN** startup completes and the user has not yet pressed the shortcut
- **THEN** the launcher window exists, is hidden, and its interface is built

#### Scenario: Content is not reloaded between invocations

- **WHEN** the launcher is shown, hidden, and shown again
- **THEN** the launcher interface has been built exactly once since application
  start

### Requirement: Launcher positioning

When shown, the launcher SHALL be positioned on the display under the mouse
pointer, centred horizontally and placed above the vertical centre of that
display. On Windows the launcher SHALL be placed within the display's work
area, which leaves out the taskbar, so that its maximum height fits inside it.
On macOS the launcher SHALL be placed against the display's full frame. On
Windows, while the launcher is shown, a height change SHALL keep its top edge
where it is and move only its bottom edge.

#### Scenario: Multi-monitor placement

- **WHEN** the mouse pointer is on a secondary display and the user presses the shortcut
- **THEN** the launcher appears on that secondary display, not the primary one

#### Scenario: Display configuration changes between invocations

- **WHEN** a display is disconnected while the launcher is hidden and the user then presses the shortcut
- **THEN** the launcher appears fully within the bounds of a currently connected display

#### Scenario: Mixed display scaling

- **WHEN** the launcher is shown on a display whose scaling factor differs from the primary display
- **THEN** the launcher renders at the correct physical size for that display with no clipping

#### Scenario: Placement on Windows

- **WHEN** the launcher is shown on Windows
- **THEN** it is centred horizontally in the work area of the display under the
  pointer and placed above its vertical centre

#### Scenario: Placement on macOS

- **WHEN** the launcher is shown on macOS
- **THEN** it is centred horizontally on the full frame of the display under
  the pointer and placed above its vertical centre

#### Scenario: The SearchBar stays put on Windows

- **WHEN** the launcher is shown on Windows and the user types until the window
  shrinks and then grows again
- **THEN** the SearchBar stays at the same place on screen throughout

#### Scenario: The SearchBar stays put on macOS

- **WHEN** the launcher is shown on macOS and the user types until the number
  of results falls and rises again
- **THEN** the window keeps its size and the SearchBar stays at the same place
  on screen throughout

### Requirement: Reset to root state on hide

Because the window is reused, returning the launcher to its initial state is
an explicit operation, and when it happens SHALL depend on why the launcher
was hidden.

A deliberate hide SHALL reset at once. Deliberate hides are an action that has
finished its work and hidden the launcher, and Escape pressed on an empty root
prompt. The next activation SHALL present the launcher as if it had just
started.

An interrupting hide SHALL hold the launcher's state for a grace period.
Interrupting hides are the global shortcut pressed while the launcher is
visible, focus lost to another application, and a click outside the launcher.
An activation within the period SHALL show the launcher as it was: the prompt
text, the pushed views, and the selected row intact, with the results for the
current prompt refreshed. When the period ends with the launcher still hidden,
the launcher SHALL reset so that the next activation presents it as if it had
just started, with the first frame already showing the root results.

A command still running at an interrupting hide SHALL keep running until the
period ends. Output it produces while hidden SHALL be shown when the launcher
is activated within the period. When the period ends with the launcher still
hidden, the command SHALL be cancelled and its later output dropped. A
deliberate hide SHALL cancel a running command at once.

A command that hides the launcher on its own behalf, to paste or to read a
selection, is neither kind of hide: the launcher SHALL reset its view state and
SHALL NOT cancel the command, whose next view brings the launcher back.

A failure message the user has not seen SHALL be the one exception to a reset.
An action that hides the launcher as part of its work can only report a failure
once the window is gone, so a failure SHALL survive the reset and be shown on
the next activation. It SHALL be cleared as soon as the user types, runs
another action, or presses Escape.

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
  shows the root results

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

- **WHEN** `launcher.reset-after` is 0 and the launcher is hidden by an
  interrupting hide
- **THEN** the next activation presents the launcher as if it had just started
- **AND** a command that was running is cancelled at the hide

#### Scenario: A command hides the launcher for its own work

- **WHEN** a command hides the launcher to paste or read a selection and then
  produces a view
- **THEN** the launcher comes back showing that view
- **AND** the command was not cancelled by the hide

#### Scenario: A failure from a dismissing action is shown afterwards

- **WHEN** an action hides the launcher and then fails
- **THEN** the next activation shows that failure's message

#### Scenario: The user clears a carried failure

- **WHEN** a carried failure is on screen and the user types, runs an action, or
  presses Escape
- **THEN** the message goes away

### Requirement: Dismissal

The launcher SHALL hide when it loses focus to another application and when the
global shortcut is pressed while it is visible. Both of these are interrupting
hides and SHALL hold the launcher's state as the reset requirement describes.

At the root prompt, Escape SHALL act in two stages. With text in the prompt,
Escape SHALL clear the prompt and leave the launcher open. With an empty
prompt, Escape SHALL hide the launcher. This is a deliberate hide. In a pushed
view, Escape SHALL pop that view, whether or not the prompt holds text, as the
view stack in view-protocol describes.

#### Scenario: Escape clears a non-empty prompt

- **WHEN** the launcher shows the root prompt with text in it and the user presses Escape
- **THEN** the prompt is cleared
- **AND** the launcher stays visible

#### Scenario: Escape on an empty prompt dismisses

- **WHEN** the launcher shows the root prompt, empty, and the user presses Escape
- **THEN** the launcher hides
- **AND** focus returns to the application that was in the foreground beforehand

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

## REMOVED Requirements

### Requirement: Activation latency

**Reason**: Its scenario "Latency is observable in development" is developer
tooling, which AGENTS.md covers (`DANGO_MEASURE`). A MODIFIED block cannot
drop a scenario, so the rule moves to a new header.

**Migration**: See "Activation feels immediate".

### Requirement: Launcher window appearance

**Reason**: The window height follows content only on Windows. macOS keeps a
fixed height, and the height scenarios need per-platform names. A MODIFIED
block cannot drop or rename a scenario, so the rule moves to a new header.

**Migration**: See "Launcher window appearance and height".

### Requirement: Quit

**Reason**: Releasing the global shortcut on exit is already in
global-activation ("Shortcut is released on exit"). A MODIFIED block cannot
drop the scenario that repeats it, so the rule moves to a new header.

**Migration**: See "Quit from the tray menu", and global-activation for the
shortcut release.

### Requirement: The launcher renders inside the Dango process

**Reason**: It named the WebView-era engines (WebView2, WebKit) and states a
stack choice, not behaviour the user relies on. AGENTS.md records that Slint
draws the interface inside the Dango process.

**Migration**: None. See the tech stack in AGENTS.md.

## ADDED Requirements

### Requirement: Activation feels immediate

Showing the launcher SHALL feel immediate on both supported platforms. The
activation path SHALL NOT create the window, build the interface, or wait on
work that could have run earlier.

#### Scenario: Cold first activation

- **WHEN** the user presses the shortcut for the first time after application start
- **THEN** the launcher is painted without visible delay

#### Scenario: Warm repeat activation

- **WHEN** the user presses the shortcut again after a previous show and hide
- **THEN** the launcher is painted without visible delay

### Requirement: Launcher window appearance and height

The launcher window SHALL be borderless, without a title bar, with a transparent
background, always on top of other windows, and non-resizable by the user.

On Windows, its height SHALL follow its content: a list or grid SHALL make the
window as tall as the SearchBar, its rows, and the ActionBar, up to 480 logical
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

### Requirement: Quit from the tray menu

The user SHALL be able to quit Dango from the tray menu. What happens to the
global shortcut on exit is in global-activation.

#### Scenario: Quit from the tray

- **WHEN** the user quits Dango from the tray menu
- **THEN** the process exits
