## Purpose

The built-in `display` extension lets the user brighten and dim their external monitors from the keyboard, over the DDC/CI protocol most monitors support, without reaching for the monitor's own buttons.

## ADDED Requirements

### Requirement: Brightness commands are searchable and invocable

The `display` extension SHALL contribute two no-view commands, titled `Increase Brightness` and `Decrease Brightness`, searchable in root search like any other command and bindable to a hotkey under `display.commands.<id>.hotkey`.

#### Scenario: Found by name

- **WHEN** the user types "bright"
- **THEN** both brightness commands appear in the results

#### Scenario: Found by keyword

- **WHEN** the user types "dim", "monitor", or "display"
- **THEN** the matching brightness commands appear in the results

#### Scenario: Extension can be disabled

- **WHEN** the `display` extension is disabled
- **THEN** neither brightness command appears in any query

#### Scenario: Run from a hotkey

- **WHEN** `Increase Brightness` has a hotkey and the user presses it while another application is focused
- **THEN** brightness changes without the launcher appearing

### Requirement: Every external monitor steps from its own level

A brightness command SHALL change every external monitor that answers DDC/CI. Each monitor SHALL move by one tenth of its own brightness range from its current level, up for `Increase Brightness` and down for `Decrease Brightness`, and SHALL stop at its minimum and maximum. Built-in laptop panels SHALL NOT be changed.

#### Scenario: Two monitors at different levels

- **GIVEN** one monitor at 30% and another at 60%
- **WHEN** the user runs `Increase Brightness`
- **THEN** the first monitor is at 40% and the second at 70%

#### Scenario: Already at the top

- **GIVEN** a monitor at 95%
- **WHEN** the user runs `Increase Brightness` twice
- **THEN** the monitor is at 100% after both runs

#### Scenario: Already at the bottom

- **GIVEN** a monitor at 0%
- **WHEN** the user runs `Decrease Brightness`
- **THEN** the monitor stays at 0% and the command still succeeds

#### Scenario: A laptop with an external monitor

- **WHEN** a laptop with one external monitor runs `Decrease Brightness`
- **THEN** the external monitor dims
- **AND** the laptop's own screen does not change

### Requirement: Repeated presses each count

Each run of a brightness command SHALL take effect, also when the user runs it again before the previous run has finished. No two runs SHALL change monitors at the same time. Presses made while a change is in progress SHALL be combined and applied together as soon as it ends, so the level catches up with the presses instead of trailing them one step at a time.

#### Scenario: Three quick presses

- **GIVEN** a monitor at 50%
- **WHEN** the user presses the `Increase Brightness` hotkey three times in quick succession
- **THEN** the monitor ends at 80%

#### Scenario: Up and down cancel out

- **GIVEN** a monitor at 50% and a change in progress
- **WHEN** the user presses `Increase Brightness` and then `Decrease Brightness` before it ends
- **THEN** the monitor ends where that change left it

### Requirement: Every change is reported as it lands

During quick presses, each change SHALL report its level in the notice as soon as it lands, replacing the previous notice, also when a newer press has come in since. A failure SHALL be reported the same way, so it is never lost behind a later press.

#### Scenario: The notice follows quick presses

- **GIVEN** a monitor at 50%
- **WHEN** the user presses the `Increase Brightness` hotkey five times in quick succession
- **THEN** the notice shows a level between 60% and 100% while the presses are still being applied
- **AND** it ends reading "Brightness set to 100%"

#### Scenario: A failure during quick presses

- **WHEN** the user presses a brightness hotkey three times in quick succession and no monitor answers DDC/CI
- **THEN** the notice shows "Turn on DDC/CI in your monitor's menu to change brightness"

### Requirement: The new level is reported

On success the command SHALL report the new level in the notice. When every changed monitor is at the same level, the message SHALL read "Brightness set to" followed by that level. When levels differ, it SHALL list each distinct level, lowest first.

#### Scenario: One level

- **WHEN** `Decrease Brightness` leaves every monitor at 60%
- **THEN** the notice reads "Brightness set to 60%"

#### Scenario: Different levels

- **WHEN** `Increase Brightness` leaves one monitor at 40% and another at 70%
- **THEN** the notice reads "Brightness set to 40% and 70%"

### Requirement: Failures say what to do

When no external monitor answers DDC/CI, the command SHALL fail with "Turn on DDC/CI in your monitor's menu to change brightness". When monitors answer but none accepts the change, it SHALL fail with "Couldn't change monitor brightness". When some monitors change and others do not, the command SHALL succeed and report the monitors that changed. Every monitor's error SHALL be written to the log, never shown.

#### Scenario: No monitor answers

- **WHEN** a brightness command runs and no external monitor answers DDC/CI, for example because DDC/CI is off in the monitor's menu
- **THEN** the failure reads "Turn on DDC/CI in your monitor's menu to change brightness"

#### Scenario: Every monitor refuses

- **WHEN** monitors answer but every one rejects the new level
- **THEN** the failure reads "Couldn't change monitor brightness"
- **AND** each monitor's error is written to the log

#### Scenario: One of two monitors refuses

- **WHEN** one monitor changes to 70% and the other rejects the change
- **THEN** the notice reads "Brightness set to 70%"
- **AND** the rejecting monitor's error is written to the log

#### Scenario: Failure from a hotkey

- **WHEN** a brightness command run from its hotkey fails
- **THEN** the launcher does not appear and the notice shows the failure
