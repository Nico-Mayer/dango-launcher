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

#### Scenario: Monitors on Windows

- **WHEN** a brightness command runs on Windows
- **THEN** every external monitor that answers DDC/CI changes, whichever graphics output it is connected to

#### Scenario: Monitors on macOS

- **WHEN** a brightness command runs on macOS, on an Intel or an Apple Silicon Mac
- **THEN** every external monitor that answers DDC/CI through its connection changes

### Requirement: Repeated presses each count

Each run of a brightness command SHALL take effect, also when the user runs it again before the previous run has finished. Runs SHALL apply in the order they were made, each from the level the previous one left.

#### Scenario: Three quick presses

- **GIVEN** a monitor at 50%
- **WHEN** the user presses the `Increase Brightness` hotkey three times in quick succession
- **THEN** the monitor ends at 80%

### Requirement: Brightness changes never block the launcher

Monitors answer DDC/CI slowly. While a brightness command talks to monitors, the launcher, its hotkey, and other hotkeys SHALL stay responsive.

#### Scenario: Opening the launcher during a change

- **WHEN** the user presses the launcher hotkey while a brightness change is still in progress
- **THEN** the launcher opens without visible delay

### Requirement: The new level is reported

On success the command SHALL report the new level in the notice. When every changed monitor is at the same level, the message SHALL read "Brightness set to" followed by that level. When levels differ, it SHALL list each distinct level, lowest first.

#### Scenario: One level

- **WHEN** `Decrease Brightness` leaves every monitor at 60%
- **THEN** the notice reads "Brightness set to 60%"

#### Scenario: Different levels

- **WHEN** `Increase Brightness` leaves one monitor at 40% and another at 70%
- **THEN** the notice reads "Brightness set to 40% and 70%"

#### Scenario: Run from root search

- **WHEN** the user runs a brightness command from root search and it succeeds
- **THEN** the launcher hides
- **AND** the notice reports the new level

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
