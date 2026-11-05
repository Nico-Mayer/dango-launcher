## ADDED Requirements

### Requirement: A command can repeat while its hotkey is held

A built-in command MAY declare that it repeats while held. Holding the hotkey of such a command SHALL run it once at the press and, after a short pause that tells a hold from a tap, again at a steady pace until the key is released. Every other command SHALL run exactly once per press, however long the key is held. A repeat SHALL also stop when another hotkey is pressed, the launcher is shown, or the config file is reloaded.

#### Scenario: A tap runs once

- **WHEN** the user taps the hotkey of a command that repeats while held
- **THEN** the command runs once

#### Scenario: Holding repeats until release

- **WHEN** the user holds the hotkey of a command that repeats while held
- **THEN** the command runs at the press, then again repeatedly while the key stays down
- **AND** it stops running when the key is released

#### Scenario: Other commands run once

- **WHEN** the user holds the hotkey of a window-management command
- **THEN** the command runs once

#### Scenario: Another hotkey ends the repeat

- **WHEN** the user holds a repeating command's hotkey and presses another hotkey
- **THEN** the repeat stops and the other hotkey's target runs

#### Scenario: Holding on Windows

- **WHEN** the user holds a repeating command's hotkey on Windows
- **THEN** the command repeats until release, although Windows reports the chord only once

#### Scenario: Holding on macOS

- **WHEN** the user holds a repeating command's hotkey on macOS
- **THEN** the command repeats until release, although macOS reports the chord only once
