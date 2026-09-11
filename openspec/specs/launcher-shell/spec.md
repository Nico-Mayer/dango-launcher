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

Because the window is reused, hiding the launcher SHALL signal the frontend to
return to its initial state. The next activation SHALL present the launcher as
if it had just started.

#### Scenario: State does not leak between invocations

- **WHEN** the user types into the prompt, hides the launcher, and shows it again
- **THEN** the prompt is empty

### Requirement: Dismissal

The launcher SHALL hide when it loses focus to another application and when the
global shortcut is pressed while it is visible.

Escape SHALL act in two stages. With text in the prompt, Escape SHALL clear the
prompt and leave the launcher open. With an empty prompt, Escape SHALL hide the
launcher.

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

#### Scenario: Shortcut toggles

- **WHEN** the launcher is visible and the user presses the global shortcut
- **THEN** the launcher hides

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
