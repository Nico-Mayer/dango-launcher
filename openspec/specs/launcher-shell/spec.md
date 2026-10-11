# launcher-shell Specification

## Purpose

The resident tray application and the launcher window it owns: how the window is
created, shown, hidden, positioned, and kept warm so that summoning it is
instant from anywhere in the operating system.

## Requirements

### Requirement: Resident tray application

Dango SHALL run as a resident background application with a system tray icon as
its only persistent user interface. It SHALL NOT appear in the dock, the
taskbar, or the application switcher.

#### Scenario: Application starts

- **WHEN** Dango is launched
- **THEN** a tray icon appears
- **AND** no window is visible
- **AND** the application does not take focus from the foreground application

#### Scenario: Tray menu is available on macOS

- **WHEN** the user clicks the tray icon on macOS
- **THEN** a menu appears offering to toggle the launcher and to quit

#### Scenario: Tray menu is available on Windows

- **WHEN** the user right-clicks the tray icon on Windows
- **THEN** a menu appears offering to toggle the launcher and to quit

#### Scenario: Application is absent from the switcher

- **WHEN** the user cycles applications with Command+Tab on macOS or Alt+Tab on Windows
- **THEN** Dango is not listed, whether the launcher window is visible or hidden

### Requirement: Warm launcher window

The launcher window SHALL be created hidden during application startup and SHALL
persist for the lifetime of the process. Showing and hiding the launcher SHALL
NOT create or destroy the window or reload its content.

#### Scenario: Window exists before first use

- **WHEN** startup completes and the user has not yet pressed the shortcut
- **THEN** the launcher window exists, is hidden, and has finished loading its content

#### Scenario: Content is not reloaded between invocations

- **WHEN** the launcher is shown, hidden, and shown again
- **THEN** the frontend has loaded exactly once since application start

### Requirement: Activation latency

Showing the launcher SHALL complete within 80 milliseconds, measured from the
global shortcut being received to the first frame of the launcher window being
painted, on both supported platforms.

#### Scenario: Cold first activation

- **WHEN** the user presses the shortcut for the first time after application start
- **THEN** the launcher is painted within 80 milliseconds

#### Scenario: Warm repeat activation

- **WHEN** the user presses the shortcut again after a previous show and hide
- **THEN** the launcher is painted within 80 milliseconds

#### Scenario: Latency is observable in development

- **WHEN** the application runs in a development build
- **THEN** each activation records its hotkey-to-paint duration where a developer can read it

### Requirement: Launcher window appearance

The launcher window SHALL be borderless, without a title bar, with a transparent
background, always on top of other windows, and non-resizable by the user.

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

### Requirement: Launcher positioning

When shown, the launcher SHALL be positioned on the display that currently holds
the foreground window, centred horizontally and placed above the vertical centre
of that display's work area.

#### Scenario: Multi-monitor placement

- **WHEN** the foreground window is on a secondary display and the user presses the shortcut
- **THEN** the launcher appears on that secondary display, not the primary one

#### Scenario: Display configuration changes between invocations

- **WHEN** a display is disconnected while the launcher is hidden and the user then presses the shortcut
- **THEN** the launcher appears fully within the bounds of a currently connected display

#### Scenario: Mixed display scaling

- **WHEN** the launcher is shown on a display whose scaling factor differs from the primary display
- **THEN** the launcher renders at the correct physical size for that display with no clipping

### Requirement: Reset to root state on hide

Because the window is reused, returning the frontend to its initial state is
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

Escape SHALL act in two stages. With text in the prompt, Escape SHALL clear the
prompt and leave the launcher open. With an empty prompt, Escape SHALL hide the
launcher. This is a deliberate hide.

#### Scenario: Escape clears a non-empty prompt

- **WHEN** the launcher is visible with text in the prompt and the user presses Escape
- **THEN** the prompt is cleared
- **AND** the launcher stays visible

#### Scenario: Escape on an empty prompt dismisses

- **WHEN** the launcher is visible with an empty prompt and the user presses Escape
- **THEN** the launcher hides
- **AND** focus returns to the application that was in the foreground beforehand

#### Scenario: Clicking away dismisses

- **WHEN** the launcher is visible and the user clicks another application
- **THEN** the launcher hides
- **AND** its state is held for the grace period

#### Scenario: Shortcut toggles

- **WHEN** the launcher is visible and the user presses the global shortcut
- **THEN** the launcher hides
- **AND** its state is held for the grace period

### Requirement: Single instance

Only one Dango process SHALL run per user session. Launching Dango while an
instance is already running SHALL show the existing instance's launcher instead
of starting a second process.

#### Scenario: Second launch is redirected

- **WHEN** Dango is already running and the user launches it again
- **THEN** no second process remains running
- **AND** the existing instance shows its launcher

### Requirement: Quit

The user SHALL be able to quit Dango from the tray menu, and quitting SHALL
release the global shortcut back to the operating system.

#### Scenario: Quit releases the shortcut

- **WHEN** the user quits Dango from the tray menu
- **THEN** the process exits
- **AND** the global shortcut is available to other applications

### Requirement: The grace period is a config setting

Dango SHALL read a whole number of seconds from `launcher.reset-after` in the
config file as the grace period for interrupting hides. When the setting is
absent, the period SHALL be 60 seconds. Zero SHALL mean an interrupting hide
resets at once. The schema SHALL reject a negative or fractional value. A
changed value SHALL apply to the next hide without a restart, and the setting
SHALL appear in the generated schema and in the settings reference.

#### Scenario: Setting is absent

- **WHEN** the config file does not mention `launcher.reset-after`
- **THEN** an interrupting hide holds state for 60 seconds

#### Scenario: Setting is changed while running

- **WHEN** the user sets `launcher.reset-after` to 300 and saves the file while
  Dango is running
- **THEN** the next interrupting hide holds state for 300 seconds, with no restart

#### Scenario: An invalid value is rejected by the schema

- **WHEN** the user sets `launcher.reset-after` to -5 or to 1.5
- **THEN** editor validation against the schema reports the value as invalid

#### Scenario: The setting is documented

- **WHEN** the settings reference is generated
- **THEN** it lists `launcher.reset-after` with its type, default, and description
