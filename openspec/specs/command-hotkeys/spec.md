# command-hotkeys Specification

## Purpose

Binding any command to a global hotkey through the config file, so a command runs
from a single keypress anywhere, and telling the user plainly when two bindings
collide rather than letting one quietly lose.

## Requirements

### Requirement: A command can be bound to a global hotkey

A command MAY declare a global hotkey in the configuration file at
`extensions.<id>.commands.<id>.hotkey`, written in the same grammar as the
launcher hotkey. Dango SHALL register the hotkey of each bound command that is
enabled, so that pressing it invokes that command regardless of which
application has focus. A disabled command's hotkey SHALL NOT be registered,
and Dango SHALL write one line to its log for each disabled command that has a
hotkey, naming the command.

A command or application hotkey MAY be one chord for both platforms or an
object with `macos` and `windows` chords. With the object, each platform SHALL
register its own chord and ignore the other's. A platform with no value SHALL
register no chord for that binding and SHALL surface it as a refused hotkey.

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

#### Scenario: A disabled command keeps no hotkey

- **WHEN** a command has a hotkey in the config and `"enabled": false`
- **THEN** its chord is not registered and pressing it does not invoke the command
- **AND** the log notes that the command has a hotkey but is disabled

#### Scenario: A per-platform command chord

- **WHEN** a command's hotkey is written as an object with `macos` and
  `windows` chords
- **THEN** each platform registers its own chord and ignores the other's

#### Scenario: A chord for the other platform only

- **WHEN** a hotkey object has a value only for the other platform
- **THEN** no chord is registered for that binding on this platform
- **AND** it is surfaced through the tray status line and the log as a refused
  hotkey, naming the command or application

### Requirement: A hotkey invokes a command the same way root search does

Pressing a command's hotkey SHALL have the same effect as selecting that command
in root search and confirming it. A command that shows no view SHALL run without
the launcher appearing; a command that shows a view SHALL show the launcher with
that view.

Sameness SHALL extend to what the user can then do: a view opened by a hotkey
SHALL accept confirmation, per-action shortcuts, and its action panel exactly as
the same view does when it is opened from root search.

#### Scenario: A no-view command runs without the launcher

- **WHEN** the user presses the hotkey for a command that shows no view
- **THEN** the command runs and the launcher does not appear

#### Scenario: A view command shows its view

- **WHEN** the user presses the hotkey for a command that shows a view
- **THEN** the launcher appears showing that view

#### Scenario: Confirming in a hotkey-opened view

- **WHEN** the user presses the hotkey for a view command and confirms a row in
  the view that appears
- **THEN** that row's primary action runs

#### Scenario: The action panel in a hotkey-opened view

- **WHEN** the user opens the action panel in a hotkey-opened view and chooses
  an action
- **THEN** that action runs

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
be reported, not applied. A disabled command takes part in no conflict.

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

#### Scenario: A disabled command shares a chord

- **WHEN** a disabled command and an enabled command are bound to the same chord
- **THEN** the enabled command takes the chord
- **AND** no conflict is surfaced

### Requirement: An application can be bound to a global hotkey

An installed application MAY be bound to a global hotkey in the configuration
file at `extensions.dango.applications.apps.<name>.hotkey`, in the same grammar
as every other hotkey. Dango SHALL register each bound chord so that pressing
it launches that application, or brings it to the front when it is already
running, regardless of which application has focus, and without the launcher
appearing. How an entry names its application, including a name per platform
and a name that matches more than one application, is in applications.

Application bindings SHALL be read at startup and re-applied when the
configuration file changes, without a restart, and SHALL take part in the same
first-wins conflict handling as command bindings.

#### Scenario: A bound chord opens the application

- **WHEN** an application has a hotkey in the config and the user presses that
  chord while another application is focused
- **THEN** that application is launched
- **AND** the launcher does not appear

#### Scenario: A bound chord brings a running application to the front

- **WHEN** the bound application is already running and the user presses its
  chord
- **THEN** its existing window comes to the front rather than a second instance
  starting

#### Scenario: Binding an application live

- **WHEN** the user adds an application hotkey to the file while Dango is
  running
- **THEN** pressing that chord opens the application, without a restart

#### Scenario: Unbinding an application live

- **WHEN** the user removes an application's hotkey from the file
- **THEN** that chord no longer opens the application

#### Scenario: An application and a command on one chord

- **WHEN** an application and a command are bound to the same chord
- **THEN** whichever comes first in the configuration's deterministic key order keeps the chord
- **AND** the collision is surfaced with both named

#### Scenario: The application is not installed on this machine

- **WHEN** a bound application's name matches nothing in the index and the user
  presses its chord
- **THEN** nothing is launched
- **AND** the missing application is surfaced through the tray status line and
  the log, naming the entry

#### Scenario: A binding for an application installed later

- **WHEN** a bound application is installed while Dango is running and the user
  then presses its chord
- **THEN** the application is launched, without editing the file or restarting

#### Scenario: A per-platform chord

- **WHEN** an application's hotkey is written as a per-platform object
- **THEN** each platform registers its own chord and ignores the other's

### Requirement: A refused command or application hotkey is surfaced

A command or application hotkey that the hotkey grammar refuses, including a
key without a modifier other than F1 to F20, SHALL NOT be registered. Dango
SHALL surface it through the tray status line and the log, naming the command
or application, and SHALL still register every other valid binding.

#### Scenario: A command bound to a bare key

- **WHEN** a command's hotkey in the config is `k`
- **THEN** the `k` key is not registered and keeps typing `k` in other applications
- **AND** the refusal is surfaced with the command named
- **AND** the other bindings still work

#### Scenario: An application bound to a lone function key

- **WHEN** an application's hotkey in the config is `f19`
- **THEN** pressing F19 alone opens that application

#### Scenario: A refused hotkey fixed live

- **WHEN** the user replaces a refused command hotkey with a valid chord while Dango is running
- **THEN** pressing the new chord invokes the command, without a restart
