# hyperkey Specification

## Purpose

CapsLock acts system-wide as the hyper modifier (Ctrl+Alt+Shift+Super), so a
single key reaches the large collision-free chord space that launcher and
command hotkeys can bind, configured entirely from the config file.

## Requirements

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

### Requirement: The hyperkey can send a key on a tap

The configuration MAY give the hyperkey a tap key. When it does, a quick
press-and-release of the hyperkey with no other key pressed while it was down
SHALL send that key. Holding the hyperkey past a short threshold, or pressing any
other key while it is down, SHALL NOT send the tap key and SHALL leave the hyper
behavior unchanged. When no tap key is configured, a tap SHALL do nothing.

The hyper modifiers SHALL still be produced on press, so that a chord pressed
quickly after the hyperkey is not missed; whether the press was a tap SHALL be
decided on release.

#### Scenario: A tap sends the tap key

- **WHEN** the hyperkey has a tap key of Escape and the user presses and releases
  it quickly without pressing another key
- **THEN** Escape is sent
- **AND** the hyperkey's own function does not occur

#### Scenario: A hold does not send the tap key

- **WHEN** the hyperkey has a tap key and the user holds it past the threshold and
  releases it without pressing another key
- **THEN** the tap key is not sent

#### Scenario: A chord does not send the tap key

- **WHEN** the hyperkey has a tap key and the user holds it and presses another
  key to form a `hyper+<key>` chord
- **THEN** that chord is invoked
- **AND** the tap key is not sent

#### Scenario: No tap key configured

- **WHEN** the hyperkey has no tap key and the user taps it
- **THEN** nothing is sent and the hyperkey's own function does not occur

### Requirement: Shift is optional in the hyper combination

The configuration MAY choose whether Shift is part of the hyper combination.
Shift SHALL be included by default (Ctrl+Alt+Shift+Super); when the configuration
turns it off, the combination SHALL be Ctrl+Alt+Super. The same set SHALL both be
emitted by the hyperkey and be what a `hyper+<key>` chord expands to, so a bound
chord always matches what the key produces.

#### Scenario: Shift excluded still fires the chord

- **WHEN** the hyperkey excludes Shift and a command is bound to `hyper+left`
- **THEN** holding the hyperkey and pressing the left arrow invokes that command,
  because both the emitted modifiers and the chord are Ctrl+Alt+Super+Left

#### Scenario: Shift excluded leaves letters unshifted under the hyperkey

- **WHEN** the hyperkey excludes Shift and the user holds it and presses a letter
  that is not bound to a chord
- **THEN** Shift is not applied to that letter

### Requirement: The hyperkey is off unless configured

Dango SHALL treat the hyperkey as absent by default. Only when the configuration
declares a hyperkey SHALL the mapped key be remapped; otherwise every key keeps
its normal function.

#### Scenario: No hyperkey configured

- **WHEN** the configuration has no hyperkey block
- **THEN** CapsLock and every other key behave normally

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
Where the platform withholds input, the hyperkey SHALL NOT modify it. On
Windows, CapsLock SHALL then behave as the system delivers it. On macOS,
CapsLock SHALL then do nothing rather than act as a lock key.

#### Scenario: A higher-integrity window on Windows

- **WHEN** the focused window belongs to a higher-integrity (elevated) process
  and Dango is not elevated
- **THEN** the hyperkey does not modify input to that window; the key behaves as
  the system delivers it there

#### Scenario: A secure input field on macOS

- **WHEN** the focused field has secure input enabled (such as a password field)
- **THEN** the hyperkey does not modify input to that field
- **AND** pressing CapsLock does nothing there and does not toggle capitals

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
