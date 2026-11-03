## MODIFIED Requirements

### Requirement: The answer streams into the launcher

Running an AI command SHALL put a view on screen that fills in as the answer
arrives, rather than waiting for the whole answer. The launcher SHALL stay
responsive throughout, and the view SHALL show that work is in progress until
the answer is complete. Until the first text arrives, the view SHALL name the
provider it is waiting for, by the name the user gave it in the config file.

#### Scenario: A view appears before the answer does

- **WHEN** an AI command using the provider named `work` is invoked
- **THEN** a view reading "Waiting for work…" is on screen at once, without
  waiting for the provider
- **AND** it is replaced by the answer as the answer arrives

#### Scenario: Text arrives progressively

- **WHEN** the model is producing a long answer
- **THEN** the text on screen grows as it arrives

#### Scenario: The launcher stays responsive while streaming

- **WHEN** an answer is streaming
- **THEN** the action panel opens and key presses are handled without delay

#### Scenario: The answer is complete

- **WHEN** the model has finished
- **THEN** the in-progress indication is gone
- **AND** the whole answer is on screen
