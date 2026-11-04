# Spec Delta

## MODIFIED Requirements

### Requirement: Empty states name the thing and the next step

An empty state SHALL have a title that says what is absent, and MAY have one
sentence saying how to fill it. It SHALL NOT be a generic phrase such as
"Nothing here" or "That did not work". An empty state shown because of an
error SHALL follow the failure requirement above.

#### Scenario: Root search finds nothing

- **WHEN** the user types a query with no matching result and gets no answer
- **THEN** the list shows "No results for" followed by the query

#### Scenario: A pushed list is empty

- **WHEN** a command's list view has no items and declared an empty state
- **THEN** the declared title and description are shown as given

#### Scenario: A pushed list is empty without a declared state

- **WHEN** a command's list view has no items and declared no empty state
- **THEN** the list shows "Nothing to show"

#### Scenario: A pushed list is filtered to nothing

- **WHEN** the user types into a pushed list and no item matches
- **THEN** the list shows "No results for" followed by the query

#### Scenario: A history is empty on first use

- **WHEN** clipboard history is opened before anything has been copied
- **THEN** the title says nothing has been copied yet
- **AND** the description says what will appear here, in one sentence
