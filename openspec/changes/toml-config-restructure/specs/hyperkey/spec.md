# Spec Delta

## MODIFIED Requirements

### Requirement: The hyperkey follows the config file without a restart

Dango SHALL apply the hyperkey from the configuration at startup, and SHALL
re-apply it when the configuration file changes, without a restart: enabling
it, disabling it, or changing its options (whether Shift is included and the
tap key).

#### Scenario: Enabling live

- **WHEN** the user sets `enabled = true` in `[hyperkey]` while Dango is running
- **THEN** CapsLock acts as the hyper modifier without a restart

#### Scenario: Disabling live

- **WHEN** the user sets `enabled = false` in `[hyperkey]`, or removes the table
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

## ADDED Requirements

### Requirement: The hyperkey is off unless enabled

Dango SHALL treat the hyperkey as off by default. Only when the `[hyperkey]`
table sets `enabled = true` SHALL CapsLock be remapped; otherwise every key
keeps its normal function. Turning it on SHALL be all it takes: the
configuration SHALL NOT name which key, and a key name left in the table SHALL
NOT change which key is remapped or stop the hyperkey from starting.

#### Scenario: No hyperkey configured

- **WHEN** the configuration has no `[hyperkey]` table
- **THEN** CapsLock and every other key behave normally

#### Scenario: A table without enabled leaves it off

- **WHEN** the `[hyperkey]` table sets options but not `enabled`
- **THEN** CapsLock and every other key behave normally

#### Scenario: Enabled with no options turns it on

- **WHEN** the `[hyperkey]` table sets only `enabled = true`
- **THEN** CapsLock acts as the hyper modifier, with Shift included and no tap key

#### Scenario: A leftover key name is ignored

- **WHEN** the `[hyperkey]` table sets `enabled = true` and still names a key, such as `capslock` or `f13`
- **THEN** CapsLock acts as the hyper modifier
- **AND** the config schema marks the key name as not allowed

## REMOVED Requirements

### Requirement: The hyperkey is off unless configured

**Reason**: Replaced by "The hyperkey is off unless enabled". A `[hyperkey]`
table no longer turns it on by being present; `enabled = true` does, the same
key every other table uses.

**Migration**: Add `enabled = true` to `[hyperkey]`.
