## ADDED Requirements

### Requirement: A snippet may declare a keyword

A snippet MAY carry a keyword, which is what expands it in place. A keyword
SHALL be optional, SHALL be unique among snippets, and SHALL be refused for a
snippet whose template needs arguments.

#### Scenario: Creating a snippet with a keyword

- **WHEN** the user completes the create form including a keyword
- **THEN** the snippet is stored with it
- **AND** typing that keyword in any application expands the snippet

#### Scenario: A snippet without a keyword still works

- **WHEN** the user leaves the keyword empty
- **THEN** the snippet is stored
- **AND** it is still found and confirmed from root search

#### Scenario: Two snippets cannot share a keyword

- **WHEN** the user saves a snippet with a keyword another snippet already has
- **THEN** it is refused with a message naming the conflict
- **AND** neither snippet is changed

#### Scenario: A snippet that asks for arguments cannot have a keyword

- **WHEN** the user gives a keyword to a snippet whose template needs arguments
- **THEN** it is refused with a message explaining that expanding in place cannot ask for them
- **AND** the snippet can still be saved without a keyword

#### Scenario: Removing a keyword

- **WHEN** the user clears a snippet's keyword and saves
- **THEN** typing the old keyword no longer expands anything
- **AND** the snippet is still found in root search

#### Scenario: A keyword is shown where the snippet is

- **WHEN** a snippet with a keyword appears in root search
- **THEN** its keyword is visible, so the user can remember what to type

#### Scenario: Removing a snippet removes its keyword

- **WHEN** a snippet is removed
- **THEN** typing its keyword no longer expands anything
