## MODIFIED Requirements

### Requirement: Every external monitor steps from its own level

A brightness command SHALL change every external monitor that answers DDC/CI, or only the one under the mouse pointer when the `display.monitors` setting says so (see "Brightness can follow the pointer"). Each monitor SHALL move by one tenth of its own brightness range from its current level, up for `Increase Brightness` and down for `Decrease Brightness`, and SHALL stop at its minimum and maximum. Built-in laptop panels SHALL NOT be changed.

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

## ADDED Requirements

### Requirement: Brightness can follow the pointer

The `display.monitors` setting SHALL choose which monitors a brightness command changes: `"all"` changes every external monitor, `"pointer"` changes only the external monitor under the mouse pointer when the change is applied. Absent means `"all"`. An edit to the setting SHALL apply from the next press, without a restart. When the monitor under the pointer does not answer DDC/CI, the command SHALL fail as when no monitor answers, and SHALL NOT change any other monitor.

#### Scenario: Default changes every monitor

- **GIVEN** `display.monitors` is not set, and two monitors at 30% and 60%
- **WHEN** the user runs `Increase Brightness`
- **THEN** the monitors are at 40% and 70%

#### Scenario: Only the monitor under the pointer changes

- **GIVEN** `display.monitors = "pointer"`, monitor A at 30% and monitor B at 60%, and the pointer on monitor B
- **WHEN** the user runs `Increase Brightness`
- **THEN** monitor B is at 70% and monitor A stays at 30%
- **AND** the notice reads "Brightness set to 70%"

#### Scenario: Moving the pointer moves the target

- **GIVEN** `display.monitors = "pointer"` and two monitors at 50%
- **WHEN** the user runs `Decrease Brightness` with the pointer on the first monitor, then moves the pointer to the second and runs it again
- **THEN** both monitors are at 40%

#### Scenario: The pointer is on a monitor without DDC/CI

- **GIVEN** `display.monitors = "pointer"` and the pointer on a monitor with DDC/CI turned off, beside another that answers
- **WHEN** the user runs `Increase Brightness`
- **THEN** the failure reads "Turn on DDC/CI in your monitor's menu to change brightness"
- **AND** the other monitor does not change

#### Scenario: The pointer is on a Windows laptop's own screen

- **GIVEN** a Windows laptop with one external monitor, `display.monitors = "pointer"`, and the pointer on the laptop's screen
- **WHEN** the user runs `Increase Brightness`
- **THEN** the failure reads "Turn on DDC/CI in your monitor's menu to change brightness"
- **AND** neither screen changes

#### Scenario: The pointer is on a MacBook's own screen

- **GIVEN** a MacBook with one external monitor, `display.monitors = "pointer"`, and the pointer on the MacBook's screen
- **WHEN** the user runs `Increase Brightness`
- **THEN** the failure reads "Turn on DDC/CI in your monitor's menu to change brightness"
- **AND** neither screen changes

#### Scenario: The setting applies without a restart

- **GIVEN** `display.monitors` is not set
- **WHEN** the user sets it to `"pointer"` in the config file, saves, and runs `Increase Brightness`
- **THEN** only the monitor under the pointer changes

#### Scenario: An unknown value is a config problem

- **WHEN** the config file sets `display.monitors = "focused"`
- **THEN** the file is invalid and the problem names `monitors`
