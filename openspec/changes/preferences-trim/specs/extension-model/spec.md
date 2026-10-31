## ADDED Requirements

### Requirement: Extension preferences

An extension SHALL be able to declare preferences at extension level, each
with a type, an optional default value, a description written for the user,
and, for a string preference that takes one of a fixed set of values, that
set. Dango SHALL supply the declared default when no value has been set, and
SHALL use the description and values wherever it describes the preference to
the user, including the config schema and the settings reference.

#### Scenario: Default is supplied

- **WHEN** a preference has never been set by the user
- **THEN** reading it returns the declared default

#### Scenario: No value and no default

- **WHEN** a preference declares no default and the user has set no value
- **THEN** reading it returns no value and the extension decides what to do

#### Scenario: Every built-in preference is described

- **WHEN** a built-in extension's manifest is validated
- **THEN** every preference it declares has a non-empty description

#### Scenario: A fixed set of values is declared once

- **WHEN** a string preference accepts only certain values, such as the clipboard's primary action
- **THEN** the declaration lists them
- **AND** reading a stored value outside the set returns the default

## REMOVED Requirements

### Requirement: Typed preferences

**Reason**: Command-level and required preferences were never implemented. No
built-in declares one, nothing reads a command-level value, and nothing
prompts for a missing required value. Replaced by "Extension preferences",
which keeps everything that is real.

**Migration**: None. Declare preferences at extension level. A
`preferences` key under a command in the config file had no effect and is no
longer part of the schema.
