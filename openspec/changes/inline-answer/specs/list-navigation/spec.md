# Spec Delta

## MODIFIED Requirements

### Requirement: A non-empty list always has exactly one selected row

Whenever a list shows at least one row, exactly one of those rows SHALL be
selected and SHALL be marked as selected on screen. An empty list SHALL have no
selection.

The selection SHALL be the row a confirmation acts on, so what the user sees
highlighted and what Enter acts on can never disagree.

In root search, an answer card (see inline-answers) SHALL count as the row
above the first result, so it can hold the selection like any row.

#### Scenario: A list opens with a selection

- **WHEN** a list is shown with at least one row
- **THEN** the first row is selected and marked as selected

#### Scenario: Replacing the rows keeps the selected item

- **WHEN** the rows are replaced for the same query, as when a slow provider's
  results arrive late, and the selected item is still among them
- **THEN** that same item stays selected

#### Scenario: Replacing the rows drops the selected item

- **WHEN** the rows are replaced and the selected item is no longer among them
- **THEN** the first row of the new list is selected

#### Scenario: A new query selects the best match

- **WHEN** the user changes the query
- **THEN** the first row of the new results is selected, even if the previously
  selected item is still among them

#### Scenario: A list becomes empty and fills again

- **WHEN** a query narrows a list to nothing and the user then deletes a
  character so rows come back
- **THEN** the first row is selected

#### Scenario: Confirming acts on the row that is marked

- **WHEN** the user confirms
- **THEN** the action runs on the row shown as selected, and on no other row

#### Scenario: An answer card is the first row

- **WHEN** root search shows an answer card and results
- **THEN** Arrow Up on the first result selects the card, and Arrow Up on the
  card leaves it selected
