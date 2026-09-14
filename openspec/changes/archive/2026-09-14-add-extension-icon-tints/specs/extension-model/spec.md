## ADDED Requirements

### Requirement: An extension may declare an icon tint

An extension's manifest MAY declare a tint: one name from a closed palette the
launcher owns. The tint belongs to the extension as a whole; a command SHALL NOT
declare its own. A manifest declaring no tint, or a name the launcher's palette
does not hold, SHALL load normally and SHALL be presented untinted.

#### Scenario: Extension declares a tint from the palette

- **WHEN** an extension whose manifest declares a tint the palette holds is loaded
- **THEN** the extension loads
- **AND** everything it contributes is presented with that tint

#### Scenario: Extension declares no tint

- **WHEN** an extension whose manifest declares no tint is loaded
- **THEN** it loads and is presented exactly as it was before tints existed

#### Scenario: Extension declares a tint the palette does not hold

- **WHEN** an extension declares a tint name outside the launcher's palette
- **THEN** the extension still loads and all of its contributions still work
- **AND** it is presented untinted rather than in an arbitrary colour

#### Scenario: A manifest written without a tint is still supported

- **WHEN** a manifest of the supported version that predates the tint field is loaded
- **THEN** it is accepted without change to its declared version
