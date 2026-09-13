## ADDED Requirements

### Requirement: The hyperkey can send a key on a tap

The configuration MAY give the hyperkey a tap key. When it does, a quick
press-and-release of the hyperkey with no other key pressed while it was down
SHALL send that key. Holding the hyperkey past a short threshold, or pressing any
other key while it is down, SHALL NOT send the tap key and SHALL leave the hyper
behavior unchanged. When no tap key is configured, a tap SHALL do nothing, and
this supersedes the earlier "a lone tap does nothing" scenario.

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
