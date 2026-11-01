## ADDED Requirements

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
