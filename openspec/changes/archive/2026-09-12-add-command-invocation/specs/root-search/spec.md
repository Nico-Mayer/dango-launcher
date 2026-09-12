## ADDED Requirements

### Requirement: Confirming a result acts on it

Root search SHALL act on the selected result when the user confirms it. A
command result SHALL be invoked; any other result SHALL have its primary action
performed. The result's remaining actions SHALL be reachable from its action
panel.

#### Scenario: Confirming a command result

- **WHEN** the user confirms a selected command result
- **THEN** that command is invoked

#### Scenario: Confirming a root item

- **WHEN** the user confirms a selected result contributed by a root items provider
- **THEN** that result's primary action is performed

#### Scenario: Choosing a secondary action

- **WHEN** the user opens a result's action panel and chooses an action other than the primary one
- **THEN** that action is performed instead of the primary one

#### Scenario: Confirming with no selection

- **WHEN** the user confirms while no result is selected, because the list is empty
- **THEN** nothing is invoked
- **AND** the launcher stays open
