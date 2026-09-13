## Purpose

Binding any command to a global hotkey through the config file, so a command runs
from a single keypress anywhere, and telling the user plainly when two bindings
collide rather than letting one quietly lose.

## ADDED Requirements

### Requirement: A command can be bound to a global hotkey

A command MAY declare a global hotkey in the configuration file at
`extensions.<id>.commands.<id>.hotkey`, written in the same grammar as the
launcher hotkey. Dango SHALL register each bound command's hotkey so that
pressing it invokes that command regardless of which application has focus.

Bindings SHALL be read at startup and SHALL be re-applied when the configuration
file changes, without a restart.

#### Scenario: A bound hotkey invokes its command

- **WHEN** a command has a hotkey in the config and the user presses that chord
  while another application is focused
- **THEN** that command is invoked

#### Scenario: Binding a command live

- **WHEN** the user adds a command hotkey to the file while Dango is running
- **THEN** pressing that chord invokes the command, without a restart

#### Scenario: Unbinding a command live

- **WHEN** the user removes a command's hotkey from the file
- **THEN** that chord no longer invokes the command

### Requirement: A hotkey invokes a command the same way root search does

Pressing a command's hotkey SHALL have the same effect as selecting that command
in root search and confirming it. A command that shows no view SHALL run without
the launcher appearing; a command that shows a view SHALL show the launcher with
that view.

#### Scenario: A no-view command runs without the launcher

- **WHEN** the user presses the hotkey for a command that shows no view
- **THEN** the command runs and the launcher does not appear

#### Scenario: A view command shows its view

- **WHEN** the user presses the hotkey for a command that shows a view
- **THEN** the launcher appears showing that view

### Requirement: A headless command acts on the window that was focused

When a hotkey invokes a command that acts on the user's current window, such as
inserting a snippet or moving a window, and the launcher does not appear, the
command SHALL act on the window that was focused when the hotkey was pressed.

#### Scenario: A snippet hotkey inserts into the focused window

- **WHEN** the user is typing in an editor and presses a snippet's hotkey
- **THEN** the snippet's text is inserted into that editor

#### Scenario: A window-management hotkey moves the focused window

- **WHEN** a window is focused and the user presses a window-management command's
  hotkey
- **THEN** that focused window is moved, not the launcher

### Requirement: Conflicting bindings are detected and surfaced

When more than one command is bound to the same chord, or a command is bound to
the launcher's chord, or the operating system refuses to register a chord, Dango
SHALL detect it and surface it through the tray status line and the log rather
than failing silently. The first binding to a chord SHALL win; the others SHALL
be reported, not applied.

Whether the operating system refuses a chord another application already owns is
platform-dependent, so Dango SHALL surface a refusal where the platform reports
one and SHALL NOT be expected to report a conflict the platform does not raise.

#### Scenario: Two commands on one chord

- **WHEN** two commands are bound to the same chord
- **THEN** the first bound command takes the chord
- **AND** the collision is surfaced with both commands named

#### Scenario: A command on the launcher chord

- **WHEN** a command is bound to the chord that summons the launcher
- **THEN** the launcher keeps the chord
- **AND** the conflict is surfaced

#### Scenario: Windows refuses a chord another application owns

- **WHEN** Windows refuses to register a bound chord because another application
  owns it
- **THEN** the failure is surfaced rather than lost
- **AND** the other bindings still work

#### Scenario: macOS accepts a chord the system already owns

- **WHEN** a chord the system already owns is bound on macOS
- **THEN** registration succeeds and no conflict is surfaced, because macOS does
  not refuse it
- **AND** pressing the chord runs the system's action rather than the bound
  command, since the system takes it first
