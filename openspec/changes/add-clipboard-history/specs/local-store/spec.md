## ADDED Requirements

### Requirement: Preference values are stored and read back

The store SHALL persist a preference value against the extension and, where the
preference is command-scoped, the command that declares it. Reading a preference
that has never been set SHALL yield the declared default rather than an absence.

#### Scenario: Value is stored and read back

- **WHEN** a preference is set and later read
- **THEN** the stored value is returned

#### Scenario: Unset preference yields its default

- **WHEN** a preference that has never been set is read
- **THEN** the default declared in the manifest is returned

#### Scenario: Extension and command scopes are separate

- **WHEN** an extension-level preference and a command-level preference share a key
- **THEN** each reads back its own value

#### Scenario: Value survives a restart

- **WHEN** Dango restarts after a preference was set
- **THEN** reading it returns the value that was set

#### Scenario: Stored value no longer fits its declared type

- **WHEN** a stored value cannot be read as the type the manifest declares
- **THEN** the declared default is returned
- **AND** the extension still loads
