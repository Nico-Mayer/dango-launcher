# Spec Delta

## MODIFIED Requirements

### Requirement: The hyperkey is off unless configured

Dango SHALL treat the hyperkey as absent by default. Only when the configuration
declares a hyperkey SHALL CapsLock be remapped; otherwise every key keeps its
normal function. Declaring a hyperkey SHALL be all it takes: the configuration
SHALL NOT name which key, and a key name left in the hyperkey block SHALL NOT
change which key is remapped or stop the hyperkey from starting.

#### Scenario: No hyperkey configured

- **WHEN** the configuration has no hyperkey block
- **THEN** CapsLock and every other key behave normally

#### Scenario: An empty hyperkey block turns it on

- **WHEN** the configuration has a hyperkey block with no options
- **THEN** CapsLock acts as the hyper modifier, with Shift included and no tap key

#### Scenario: A leftover key name is ignored

- **WHEN** the hyperkey block still names a key, such as `capslock` or `f13`
- **THEN** CapsLock acts as the hyper modifier
- **AND** the config schema marks the key name as not allowed
