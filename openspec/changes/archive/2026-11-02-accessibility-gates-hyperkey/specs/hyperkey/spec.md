## MODIFIED Requirements

### Requirement: The hyperkey follows the config file without a restart

Dango SHALL apply the hyperkey from the configuration at startup, and SHALL
re-apply it when the configuration file changes, without a restart: enabling
it, disabling it, or changing its options (whether Shift is included and the
tap key).

#### Scenario: Enabling live

- **WHEN** the user adds a hyperkey to the file while Dango is running
- **THEN** CapsLock acts as the hyper modifier without a restart

#### Scenario: Disabling live

- **WHEN** the user removes the hyperkey from the file
- **THEN** CapsLock returns to its normal function

#### Scenario: Changing an option live

- **WHEN** the user changes the hyperkey's Shift setting or tap key in the file
  while Dango is running
- **THEN** the hyperkey uses the new setting without a restart

#### Scenario: The permission is missing on macOS

- **WHEN** a hyperkey is configured and Input Monitoring has not been granted
- **THEN** CapsLock keeps its normal function
- **AND** `dango.log` says the hyperkey needs Input Monitoring
- **AND** the launcher shows a warning that Input Monitoring is off and names the hyperkey

#### Scenario: Accessibility is missing on macOS

- **WHEN** a hyperkey is configured, Input Monitoring is granted, and Accessibility is not
- **THEN** CapsLock keeps its normal function
- **AND** `dango.log` says the hyperkey needs Accessibility
- **AND** the launcher shows a warning that Accessibility is off and names the hyperkey

#### Scenario: The permission is granted on macOS

- **WHEN** a hyperkey is configured, Accessibility is granted, the user grants Input Monitoring, and Dango restarts as macOS requires
- **THEN** CapsLock acts as the hyper modifier

#### Scenario: Windows needs no permission

- **WHEN** a hyperkey is configured on Windows
- **THEN** it applies at startup with no permission step
