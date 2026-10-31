## MODIFIED Requirements

### Requirement: A command can be bound to a global hotkey

A command MAY declare a global hotkey in the configuration file at
`extensions.<id>.commands.<id>.hotkey`, written in the same grammar as the
launcher hotkey. Dango SHALL register the hotkey of each bound command that is
enabled, so that pressing it invokes that command regardless of which
application has focus. A disabled command's hotkey SHALL NOT be registered.

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
