## MODIFIED Requirements

### Requirement: The answer streams into the launcher

Running an AI command SHALL put a view on screen that fills in as the answer
arrives, rather than waiting for the whole answer. The launcher SHALL stay
responsive throughout, and the view SHALL show that work is in progress until
the answer is complete.

#### Scenario: A view appears before the answer does

- **WHEN** an AI command is invoked
- **THEN** a view showing that it is working is on screen at once, without
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

### Requirement: A running request can be abandoned

The user SHALL be able to stop a request that is running. Escape SHALL leave the
view the way it leaves any view, and an abandoned request's later output SHALL
NOT appear. When the launcher session ends, the request SHALL be abandoned;
when that happens is in `openspec/specs/launcher-shell`.

#### Scenario: Escape abandons the request

- **WHEN** an answer is streaming and the user presses Escape
- **THEN** the view is closed
- **AND** no further text from that request is shown

#### Scenario: Hiding the launcher abandons the request

- **WHEN** an answer is streaming and the launcher session ends, through a
  deliberate hide or the end of the hold after an interrupting hide
- **THEN** the request is abandoned and nothing from it is shown later

#### Scenario: Running another command supersedes it

- **WHEN** the user runs a second command while an answer is still streaming
- **THEN** only the second command's output is shown
