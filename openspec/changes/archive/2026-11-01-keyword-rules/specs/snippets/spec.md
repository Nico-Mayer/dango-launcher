## MODIFIED Requirements

### Requirement: A snippet may declare a keyword

A snippet MAY carry a keyword, which is what expands it in place. A keyword
SHALL be optional, SHALL be unique among snippets, and SHALL be refused for a
snippet whose template needs arguments. A keyword SHALL also be refused when it
starts with another snippet's keyword or another snippet's keyword starts with
it, because the shorter one would always expand first.

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

#### Scenario: A keyword cannot start with another keyword

- **WHEN** another snippet has the keyword `;s` and the user saves a snippet with the keyword `;sig`
- **THEN** it is refused with "One keyword can't be the start of another, so choose a different keyword"
- **AND** neither snippet is changed

#### Scenario: A keyword cannot be the start of another keyword

- **WHEN** another snippet has the keyword `;sig` and the user saves a snippet with the keyword `;s`
- **THEN** it is refused with "One keyword can't be the start of another, so choose a different keyword"
- **AND** neither snippet is changed

#### Scenario: Keywords that differ only in case do not conflict

- **WHEN** another snippet has the keyword `;s` and the user saves a snippet with the keyword `;Sig`
- **THEN** the snippet is stored, since keywords match only with their exact case

#### Scenario: A snippet keeps its own keyword when edited

- **WHEN** the user edits a snippet and saves it with its current keyword, or with a longer keyword that starts with its current one
- **THEN** its own old keyword is not counted as a conflict

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
