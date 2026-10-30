## MODIFIED Requirements

### Requirement: Observation follows the extension's enabled state

Observation SHALL exist only while the snippets extension is enabled, and
disabling it SHALL remove the observation rather than leave it running.

#### Scenario: Disabling stops observation

- **WHEN** the user disables the snippets extension
- **THEN** nothing is observed at all
- **AND** no keyword expands

#### Scenario: Enabling starts observation

- **WHEN** the user enables the snippets extension
- **THEN** keywords expand again without a restart

#### Scenario: The permission is missing on macOS

- **WHEN** the Input Monitoring permission has not been granted
- **THEN** expansion does not work
- **AND** the launcher shows a warning that Input Monitoring is off and names keyword expansion

#### Scenario: The permission is granted on macOS

- **WHEN** the snippets extension is enabled, the user grants Input Monitoring, and Dango restarts as macOS requires
- **THEN** keywords expand

#### Scenario: Windows needs no permission

- **WHEN** the snippets extension is enabled on Windows
- **THEN** keywords expand with no permission step
