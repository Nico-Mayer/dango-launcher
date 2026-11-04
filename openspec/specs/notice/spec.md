# Notice Specification

## Purpose

The notice tells the user about an outcome they cannot see while the launcher is
hidden: a small pill that shows briefly and never gets in the way.

## Requirements

### Requirement: The notice reports only outcomes the user cannot see

While the launcher is hidden, Dango SHALL show the notice for a failure, for a
copy that closed the launcher, and for a background task that finishes or
comes to need input. It SHALL NOT show the notice for an outcome that is visible
on its own, such as a moved window, a launched application, or a locked
screen. While the launcher is visible, its ActionBar ticker SHALL report instead
and the notice SHALL NOT show.

#### Scenario: A hotkey command fails

- **WHEN** the user presses the hotkey for a no-view command and the command
  fails
- **THEN** the notice shows the failure's message
- **AND** the launcher does not appear

#### Scenario: An action fails after hiding the launcher

- **WHEN** an action hides the launcher to paste and pasting fails
- **THEN** the notice shows the failure's message

#### Scenario: A held command fails while hidden

- **WHEN** a command keeps running after an interrupting hide and fails before
  the launcher is shown again
- **THEN** the notice shows the failure's message

#### Scenario: A copy closes the launcher

- **WHEN** the user runs an action that copies to the clipboard and hides the
  launcher
- **THEN** the notice reads "Copied to clipboard"

#### Scenario: A background task finishes while hidden

- **WHEN** the AI command "Summarize" finishes in the background while the
  launcher is hidden
- **THEN** a success notice reads "Answer ready: Summarize"

#### Scenario: A visible outcome shows nothing

- **WHEN** the user presses the hotkey for a window-management command and the
  window moves
- **THEN** no notice shows

#### Scenario: The launcher is visible

- **WHEN** an action fails while the launcher is on screen
- **THEN** the failure shows as a ticker in the ActionBar
- **AND** no notice shows

### Requirement: The notice is a pill with a dot and one line

The notice SHALL look like a ticker on its own: a success dot or a failure dot, as
the ticker uses, followed by one line of text, in a rounded pill that fits its
text up to the ticker's width. Every colour, size, and radius SHALL come from the
theme, in dark and light. Its text SHALL follow the same copy rules as a ticker's.
Assistive technology SHALL be told the text and whether it reports a failure.

#### Scenario: A failure notice

- **WHEN** the notice reports a failure
- **THEN** it shows the danger dot and the failure's one sentence

#### Scenario: A success notice

- **WHEN** the notice reports a copy
- **THEN** it shows a dot in the done role and "Copied to clipboard"

#### Scenario: The notice follows the system theme

- **WHEN** the operating system switches between dark and light
- **THEN** the next notice uses the matching theme values

### Requirement: The notice disappears on its own

A success notice SHALL disappear after about two seconds and a failure notice after
about four. A new notice SHALL replace the one showing and start its own time.
Showing the launcher SHALL remove the notice at once. When the user opens the
launcher, with its hotkey or the tray menu, while the notice reports a result that
offers "Show answer", the launcher SHALL open on that answer. The notice SHALL NOT
wait for a key press or a click.

#### Scenario: A success notice times out

- **WHEN** a success notice shows
- **THEN** it is gone after about two seconds

#### Scenario: A failure notice times out

- **WHEN** a failure notice shows
- **THEN** it stays about four seconds and then is gone

#### Scenario: A newer notice replaces the old one

- **WHEN** a notice is showing and another outcome is reported
- **THEN** only the newer notice shows, for its own full time

#### Scenario: The launcher opens over a notice

- **WHEN** a notice that offers no answer is showing and the user opens the
  launcher
- **THEN** the notice is gone and the launcher shows without the notice's message

#### Scenario: The launcher opens over an answer's notice

- **WHEN** the notice reads "Answer ready: Summarize" and the user opens the
  launcher before it is gone
- **THEN** the notice is gone and the answer's view opens, with actions to paste
  or copy it and no ticker

#### Scenario: A command hotkey during an answer's notice

- **WHEN** the notice reads "Answer ready: Summarize" and the user presses the
  hotkey of a command that shows the launcher
- **THEN** that command runs and the answer does not open

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

### Requirement: The notice never takes focus or input

Showing the notice SHALL NOT change the foreground window or keyboard focus. Mouse
input over it SHALL reach the window beneath. It SHALL NOT appear in the
taskbar, Alt+Tab, the Dock, or Cmd+Tab.

#### Scenario: Typing continues on Windows

- **WHEN** the user is typing in another application on Windows and a notice shows
- **THEN** every key still reaches that application and its window stays in
  front

#### Scenario: Typing continues on macOS

- **WHEN** the user is typing in another application on macOS and a notice shows
- **THEN** every key still reaches that application and it stays the active
  application

#### Scenario: Clicks pass through

- **WHEN** the user clicks where the notice is showing
- **THEN** the click reaches the window beneath

#### Scenario: Not listed as a window on Windows

- **WHEN** a notice is showing on Windows
- **THEN** it is not in the taskbar or in Alt+Tab

#### Scenario: Not listed as a window on macOS

- **WHEN** a notice is showing on macOS
- **THEN** Dango does not appear in the Dock or in Cmd+Tab because of it
