## MODIFIED Requirements

### Requirement: A configured key acts as the hyper modifier

When the configuration names a hyperkey, Dango SHALL make CapsLock behave
system-wide as the hyper modifier: while it is held, the four hyper modifiers
(Control, Alt, Shift, Super) SHALL be in effect, so that pressing another key
produces that key combined with all four. CapsLock SHALL be the only key that
can be the hyperkey. Its own default function SHALL be suppressed while it acts
as the hyperkey.

The hyperkey SHALL only produce the modifier combination. It SHALL NOT register
any chord or decide any action; a `hyper+<key>` chord fires through the same
registration the launcher and command hotkeys already use.

#### Scenario: Holding the hyperkey reaches a bound chord

- **WHEN** a command is bound to `hyper+left` and the user holds the hyperkey and
  presses the left arrow while another application is focused
- **THEN** that command is invoked, exactly as if Ctrl+Alt+Shift+Super+Left had
  been pressed

#### Scenario: The hyperkey's original function is suppressed

- **WHEN** the hyperkey is on and the user presses CapsLock
- **THEN** CapsLock does not toggle and no capitalization state changes

#### Scenario: Other keys are unaffected

- **WHEN** the hyperkey is enabled and the user types normally without holding it
- **THEN** every other key produces its normal character or function unchanged

### Requirement: The hyperkey is bounded by what the platform lets it see

The hyperkey SHALL act only on input the operating system delivers to Dango.
Where the platform withholds input, the hyperkey SHALL NOT modify it. On
Windows, CapsLock SHALL then behave as the system delivers it. On macOS,
CapsLock is remapped below the level Dango sees, so there it SHALL do nothing
rather than act as a lock key.

#### Scenario: A higher-integrity window on Windows

- **WHEN** the focused window belongs to a higher-integrity (elevated) process
  and Dango is not elevated
- **THEN** the hyperkey does not modify input to that window; the key behaves as
  the system delivers it there

#### Scenario: A secure input field on macOS

- **WHEN** the focused field has secure input enabled (such as a password field)
- **THEN** the hyperkey does not modify input to that field
- **AND** pressing CapsLock does nothing there and does not toggle capitals

## REMOVED Requirements

### Requirement: The hyperkey applies at startup and live on a config change

**Reason**: It spoke of changing which key is mapped, but CapsLock is the only
key the hyperkey can use.

**Migration**: Replaced by "The hyperkey follows the config file without a
restart", which covers the options that can change.

## ADDED Requirements

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

#### Scenario: The permission is granted on macOS

- **WHEN** a hyperkey is configured, the user grants Input Monitoring, and Dango restarts as macOS requires
- **THEN** CapsLock acts as the hyper modifier

#### Scenario: Windows needs no permission

- **WHEN** a hyperkey is configured on Windows
- **THEN** it applies at startup with no permission step
