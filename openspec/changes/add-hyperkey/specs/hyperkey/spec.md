## Purpose

A configured physical key acts system-wide as the hyper modifier
(Ctrl+Alt+Shift+Super), so a single key reaches the large collision-free chord
space that launcher and command hotkeys can bind, configured entirely from the
config file.

## ADDED Requirements

### Requirement: A configured key acts as the hyper modifier

When the configuration names a hyperkey, Dango SHALL make that physical key
behave system-wide as the hyper modifier: while it is held, the four hyper
modifiers (Control, Alt, Shift, Super) SHALL be in effect, so that pressing
another key produces that key combined with all four. The key's own default
function SHALL be suppressed while it acts as the hyperkey.

The hyperkey SHALL only produce the modifier combination. It SHALL NOT register
any chord or decide any action; a `hyper+<key>` chord fires through the same
registration the launcher and command hotkeys already use.

#### Scenario: Holding the hyperkey reaches a bound chord

- **WHEN** a command is bound to `hyper+left` and the user holds the hyperkey and
  presses the left arrow while another application is focused
- **THEN** that command is invoked, exactly as if Ctrl+Alt+Shift+Super+Left had
  been pressed

#### Scenario: The hyperkey's original function is suppressed

- **WHEN** the hyperkey is CapsLock and the user presses it
- **THEN** CapsLock does not toggle and no capitalization state changes

#### Scenario: A lone tap does nothing

- **WHEN** the user presses and releases the hyperkey without pressing another key
- **THEN** no chord is invoked and the key's original function does not occur

#### Scenario: Other keys are unaffected

- **WHEN** the hyperkey is enabled and the user types normally without holding it
- **THEN** every other key produces its normal character or function unchanged

### Requirement: The hyperkey is off unless configured

Dango SHALL treat the hyperkey as absent by default. Only when the configuration
declares a hyperkey SHALL the mapped key be remapped; otherwise every key keeps
its normal function.

#### Scenario: No hyperkey configured

- **WHEN** the configuration has no hyperkey block
- **THEN** CapsLock and every other key behave normally

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

### Requirement: The hyperkey does not disrupt Dango's own key injection

Dango's own synthesized input, such as a snippet paste, SHALL NOT be re-processed
by the hyperkey, so enabling the hyperkey SHALL NOT change how Dango's commands
insert text.

#### Scenario: A snippet paste still works with the hyperkey on

- **WHEN** the hyperkey is enabled and a command inserts text into the focused
  window
- **THEN** the text is inserted exactly as it is with the hyperkey off

### Requirement: The hyperkey is bounded by what the platform lets it see

The hyperkey SHALL act only on input the operating system delivers to Dango.
Where the platform withholds input, the mapped key SHALL fall back to its normal
behavior rather than failing in a hidden way.

#### Scenario: A higher-integrity window on Windows

- **WHEN** the focused window belongs to a higher-integrity (elevated) process
  and Dango is not elevated
- **THEN** the hyperkey does not modify input to that window; the key behaves as
  the system delivers it there

#### Scenario: A secure input field on macOS

- **WHEN** the focused field has secure input enabled (such as a password field)
- **THEN** the hyperkey does not modify input to that field
