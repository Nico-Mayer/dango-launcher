## MODIFIED Requirements

### Requirement: The hyperkey applies at startup and live on a config change

Dango SHALL apply the hyperkey from the configuration at startup, and SHALL
re-apply it when the configuration file changes, without a restart: enabling it,
disabling it, or changing which key is mapped.

#### Scenario: Enabling live

- **WHEN** the user adds a hyperkey to the file while Dango is running
- **THEN** the mapped key acts as the hyper modifier without a restart

#### Scenario: Disabling live

- **WHEN** the user removes the hyperkey from the file
- **THEN** the previously mapped key returns to its normal function

#### Scenario: Changing the mapped key live

- **WHEN** the user changes the hyperkey from one key to another in the file
- **THEN** the old key returns to normal and the new key becomes the hyperkey

#### Scenario: The permission is missing on macOS

- **WHEN** a hyperkey is configured and Input Monitoring has not been granted
- **THEN** the mapped key keeps its normal function
- **AND** `dango.log` says the hyperkey needs Input Monitoring
- **AND** the launcher shows a warning that Input Monitoring is off and names the hyperkey

#### Scenario: The permission is granted on macOS

- **WHEN** a hyperkey is configured, the user grants Input Monitoring, and Dango restarts as macOS requires
- **THEN** the mapped key acts as the hyper modifier

#### Scenario: Windows needs no permission

- **WHEN** a hyperkey is configured on Windows
- **THEN** it applies at startup with no permission step
