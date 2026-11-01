## MODIFIED Requirements

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
