## MODIFIED Requirements

### Requirement: Typed preferences

An extension SHALL be able to declare preferences at extension level and at
command level, each with a type, a default value, whether it is required, a
description written for the user, and, for a string preference that takes one
of a fixed set of values, that set. Dango SHALL supply the declared default
when no value has been set, and SHALL use the description and values wherever
it describes the preference to the user, including the config schema and the
settings reference.

#### Scenario: Default is supplied

- **WHEN** a preference has never been set by the user
- **THEN** reading it returns the declared default

#### Scenario: Required preference is missing

- **WHEN** a command declares a required preference with no default and no value is set
- **THEN** invoking that command prompts for the value instead of running

#### Scenario: Every built-in preference is described

- **WHEN** a built-in extension's manifest is validated
- **THEN** every preference it declares has a non-empty description

#### Scenario: A fixed set of values is declared once

- **WHEN** a string preference accepts only certain values, such as the clipboard's primary action
- **THEN** the declaration lists them
- **AND** reading a stored value outside the set returns the default
