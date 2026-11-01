## ADDED Requirements

### Requirement: The hotkey pattern follows the modifier rule

The schema's hotkey pattern SHALL accept exactly the chords the hotkey parser
accepts: one or more modifiers followed by one key, or a function key from F1
to F20 alone. The settings reference SHALL state the same rule.

#### Scenario: A bare letter does not match

- **WHEN** a config sets a hotkey to `a` or `space`
- **THEN** the schema reports the value as not matching the hotkey pattern
- **AND** the parser refuses the same value

#### Scenario: A lone function key matches

- **WHEN** a config sets a hotkey to `f1` or `f20`
- **THEN** the schema accepts the value
- **AND** the parser accepts the same value

#### Scenario: A function key past F20 does not match

- **WHEN** a config sets a hotkey to `f21` or `ctrl+f24`
- **THEN** the schema reports the value as not matching the hotkey pattern
- **AND** the parser refuses the same value

#### Scenario: The reference states the rule

- **WHEN** the settings reference is generated
- **THEN** its hotkey section says a chord needs a modifier unless its key is F1 to F20
