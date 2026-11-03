# hud Specification

## Purpose

The HUD tells the user about an outcome they cannot see while the launcher is
hidden: a small pill that shows briefly and never gets in the way.

## Requirements

### Requirement: The HUD reports only outcomes the user cannot see

While the launcher is hidden, Dango SHALL show the HUD for a failure and for a
copy that closed the launcher. It SHALL NOT show the HUD for an outcome that is
visible on its own, such as a moved window, a launched application, or a locked
screen. While the launcher is visible, its ActionBar toast SHALL report instead
and the HUD SHALL NOT show.

#### Scenario: A hotkey command fails

- **WHEN** the user presses the hotkey for a no-view command and the command
  fails
- **THEN** the HUD shows the failure's message
- **AND** the launcher does not appear

#### Scenario: An action fails after hiding the launcher

- **WHEN** an action hides the launcher to paste and pasting fails
- **THEN** the HUD shows the failure's message

#### Scenario: A held command fails while hidden

- **WHEN** a command keeps running after an interrupting hide and fails before
  the launcher is shown again
- **THEN** the HUD shows the failure's message

#### Scenario: A copy closes the launcher

- **WHEN** the user runs an action that copies to the clipboard and hides the
  launcher
- **THEN** the HUD reads "Copied to clipboard"

#### Scenario: A visible outcome shows nothing

- **WHEN** the user presses the hotkey for a window-management command and the
  window moves
- **THEN** no HUD shows

#### Scenario: The launcher is visible

- **WHEN** an action fails while the launcher is on screen
- **THEN** the failure shows as a toast in the ActionBar
- **AND** no HUD shows

### Requirement: The HUD is a pill with a dot and one line

The HUD SHALL look like a toast on its own: a success dot or a failure dot, as
the toast uses, followed by one line of text, in a rounded pill that fits its
text up to the toast's width. Every colour, size, and radius SHALL come from the
theme, in dark and light. Its text SHALL follow the same copy rules as a toast's.
Assistive technology SHALL be told the text and whether it reports a failure.

#### Scenario: A failure HUD

- **WHEN** the HUD reports a failure
- **THEN** it shows the danger dot and the failure's one sentence

#### Scenario: A success HUD

- **WHEN** the HUD reports a copy
- **THEN** it shows the accent dot and "Copied to clipboard"

#### Scenario: The HUD follows the system theme

- **WHEN** the operating system switches between dark and light
- **THEN** the next HUD uses the matching theme values

### Requirement: The HUD disappears on its own

A success HUD SHALL disappear after about two seconds and a failure HUD after
about four. A new HUD SHALL replace the one showing and start its own time.
Showing the launcher SHALL remove the HUD at once. The HUD SHALL NOT wait for a
key press or a click.

#### Scenario: A success HUD times out

- **WHEN** a success HUD shows
- **THEN** it is gone after about two seconds

#### Scenario: A failure HUD times out

- **WHEN** a failure HUD shows
- **THEN** it stays about four seconds and then is gone

#### Scenario: A newer HUD replaces the old one

- **WHEN** a HUD is showing and another outcome is reported
- **THEN** only the newer HUD shows, for its own full time

#### Scenario: The launcher opens over a HUD

- **WHEN** a HUD is showing and the user opens the launcher
- **THEN** the HUD is gone and the launcher shows without the HUD's message

### Requirement: The HUD sits at the bottom of the active screen

The HUD SHALL show centred horizontally near the bottom of the work area of the
display under the mouse pointer, the display the launcher would open on, above
the taskbar or the Dock. It SHALL show over every other window, including
fullscreen applications.

#### Scenario: Placement on Windows

- **WHEN** the HUD shows on Windows with the mouse pointer on a second monitor
- **THEN** it is centred near the bottom of that monitor's work area, above the
  taskbar

#### Scenario: Placement on macOS

- **WHEN** the HUD shows on macOS with the mouse pointer on a second display
- **THEN** it is centred near the bottom of that display's visible frame, above
  the Dock

#### Scenario: Over a fullscreen application on Windows

- **WHEN** a fullscreen application is in front on Windows and the HUD shows
- **THEN** the HUD is visible over it

#### Scenario: Over a fullscreen application on macOS

- **WHEN** a fullscreen application is in front in its own Space on macOS and
  the HUD shows
- **THEN** the HUD is visible over it, in that Space

### Requirement: The HUD never takes focus or input

Showing the HUD SHALL NOT change the foreground window or keyboard focus. Mouse
input over it SHALL reach the window beneath. It SHALL NOT appear in the
taskbar, Alt+Tab, the Dock, or Cmd+Tab.

#### Scenario: Typing continues on Windows

- **WHEN** the user is typing in another application on Windows and a HUD shows
- **THEN** every key still reaches that application and its window stays in
  front

#### Scenario: Typing continues on macOS

- **WHEN** the user is typing in another application on macOS and a HUD shows
- **THEN** every key still reaches that application and it stays the active
  application

#### Scenario: Clicks pass through

- **WHEN** the user clicks where the HUD is showing
- **THEN** the click reaches the window beneath

#### Scenario: Not listed as a window on Windows

- **WHEN** a HUD is showing on Windows
- **THEN** it is not in the taskbar or in Alt+Tab

#### Scenario: Not listed as a window on macOS

- **WHEN** a HUD is showing on macOS
- **THEN** Dango does not appear in the Dock or in Cmd+Tab because of it
