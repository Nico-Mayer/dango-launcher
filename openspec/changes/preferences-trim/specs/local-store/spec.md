## REMOVED Requirements

### Requirement: Preference values are stored and read back

**Reason**: Command-scoped preference values were removed; no built-in reads one.
**Migration**: None. A preference value is stored per extension only, see "Extension preference values are stored and read back".

## ADDED Requirements

### Requirement: Extension preference values are stored and read back

The store SHALL persist a preference value against the extension that declares
it. Reading a preference that has never been set SHALL yield the declared
default, or nothing when the preference declares no default.

#### Scenario: Value is stored and read back

- **WHEN** a preference is set and later read
- **THEN** the stored value is returned

#### Scenario: Unset preference yields its default

- **WHEN** a preference that has never been set is read
- **THEN** the default declared in the manifest is returned

#### Scenario: Value survives a restart

- **WHEN** Dango restarts after a preference was set
- **THEN** reading it returns the value that was set

#### Scenario: Stored value no longer fits its declared type

- **WHEN** a stored value cannot be read as the type the manifest declares
- **THEN** the declared default is returned
- **AND** the extension still loads
