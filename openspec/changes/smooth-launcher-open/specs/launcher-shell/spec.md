## ADDED Requirements

### Requirement: Showing never paints old content

When the launcher is shown, no frame drawn before the hide SHALL be visible.
The launcher SHALL stay invisible until its first frame with the current
state is drawn, and SHALL then appear. This holds after a reset and after a
held show alike.

#### Scenario: Show after a reset on macOS

- **GIVEN** macOS, and the launcher was hidden with a pushed view and was reset
  while hidden
- **WHEN** the user activates the launcher
- **THEN** no frame shows the pushed view
- **AND** the first visible frame shows the root view

#### Scenario: Show after a reset on Windows

- **GIVEN** Windows, and the launcher was hidden with a pushed view and was
  reset while hidden
- **WHEN** the user activates the launcher
- **THEN** no frame shows the pushed view
- **AND** the first visible frame shows the root view

### Requirement: The launcher opens with an animation

Each show after the first since Dango started SHALL fade the launcher in and
slide the pill and card a short way down into place, after the first current
frame is drawn. The first show SHALL appear without animation. The animation SHALL
be short and SHALL NOT delay input: keys pressed during it SHALL act at once.
It SHALL start with the first fresh frame, so activation still feels
immediate. Hiding SHALL stay instant, with no animation.

#### Scenario: Fade and slide on macOS

- **GIVEN** macOS
- **WHEN** the user activates the launcher
- **THEN** the launcher fades in while the pill and card slide down into place

#### Scenario: Slide on Windows

- **GIVEN** Windows
- **WHEN** the user activates the launcher
- **THEN** the pill and card slide down into place

#### Scenario: The first show has no animation

- **WHEN** the user activates the launcher for the first time after Dango
  starts
- **THEN** the launcher appears in place, with no fade and no slide

#### Scenario: Typing during the animation

- **WHEN** the user activates the launcher and types `fi` before the animation
  ends
- **THEN** the prompt holds `fi` and results for `fi` appear as usual

#### Scenario: Showing again during the animation

- **WHEN** the user hides the launcher and shows it again before the
  animation ends
- **THEN** the launcher shows no old content and the animation starts again

#### Scenario: Hide is instant

- **WHEN** the user presses Escape on an empty root prompt with the list closed
- **THEN** the launcher disappears in the next frame, with no animation

### Requirement: The open list at start is a config setting

Dango SHALL read a boolean from `launcher.open-list` in the config file. When
`true`, every reset and the first show after Dango starts SHALL present the
root view with the empty-query list open (see root-search). When absent, it
SHALL mean `false`. A held show SHALL NOT be affected. A changed value SHALL
apply to the next reset without a restart, and the setting SHALL appear in the
generated schema and in the settings reference.

#### Scenario: Setting is absent

- **WHEN** the config file does not mention `launcher.open-list` and the
  launcher opens after a reset
- **THEN** the root view is closed

#### Scenario: Setting is on

- **WHEN** `launcher.open-list` is `true` and the launcher opens after a reset
- **THEN** the empty-query list is open

#### Scenario: A held show keeps its state

- **WHEN** `launcher.open-list` is `true`, the user closes the list with
  Escape, and focus moves to another application
- **AND** the user activates the launcher within the grace period
- **THEN** the list is still closed

#### Scenario: Setting is changed while running

- **WHEN** the user sets `launcher.open-list` to `true` and saves the file
  while Dango is running
- **THEN** the launcher opens with the list open after the next reset, with no
  restart

#### Scenario: Escape stages still apply

- **WHEN** `launcher.open-list` is `true` and the launcher opens after a reset
- **AND** the user presses Escape
- **THEN** the list closes and the launcher stays open
- **AND** a second Escape hides the launcher

#### Scenario: The setting is documented

- **WHEN** the settings reference is generated
- **THEN** it lists `launcher.open-list` with its type, default, and
  description

## MODIFIED Requirements

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
just started, with the first frame already showing the root view as
`launcher.open-list` sets it (see root-search).

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
activation, whether the launcher was reset or held. The notice shows it instead
(see `notice`). A background task is the one exception: its result is shown on
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

- **WHEN** the launcher is hidden by an interrupting hide, `launcher.open-list`
  is absent, and the user activates it after the grace period has passed
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
- **THEN** the notice shows that failure's message at once
- **AND** the next activation shows no failure ticker

#### Scenario: The user clears a carried failure

- **WHEN** a failure was reported while the launcher was held by an interrupting
  hide, and the user shows the launcher within the grace period
- **THEN** the held view is back without a failure ticker, because the notice
  already showed the message

#### Scenario: A background task survives the grace period

- **WHEN** an update downloads, the launcher is hidden, and the grace period
  passes
- **THEN** the download keeps going and its view is dropped

