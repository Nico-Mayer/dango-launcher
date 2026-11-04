# Spec Delta

## MODIFIED Requirements

### Requirement: A non-empty list always has exactly one selected row

Whenever a list shows at least one row, exactly one of those rows SHALL be
selected and SHALL be marked as selected on screen. An empty list SHALL have no
selection.

The selection SHALL be the row a confirmation acts on, so what the user sees
highlighted and what Enter acts on can never disagree.

In root search, an answer card (see inline-answers) SHALL count as the row
above the first result, under every rule here for rows.

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

### Requirement: Arrow keys move the selection and nothing else

Arrow Up and Arrow Down SHALL move the selection within the list by one row.
They SHALL NOT move the text cursor in the prompt, SHALL NOT move focus to the
prompt or to any other element, and SHALL NOT leave the list without a
selection.

The selection SHALL NOT wrap: Arrow Up on the first row and Arrow Down on the
last row leave the selection where it is.

#### Scenario: Arrow Down moves down one row

- **WHEN** the user presses Arrow Down with a row other than the last selected
- **THEN** the next row becomes selected

#### Scenario: Arrow Up moves up one row

- **WHEN** the user presses Arrow Up with a row other than the first selected
- **THEN** the previous row becomes selected

#### Scenario: Arrow Up on the first row stays there

- **WHEN** the user presses Arrow Up with the first row selected
- **THEN** the first row stays selected
- **AND** the prompt does not take the selection or the highlight

#### Scenario: Arrow Down on the last row stays there

- **WHEN** the user presses Arrow Down with the last row selected
- **THEN** the last row stays selected

#### Scenario: Arrows do not disturb the text cursor

- **WHEN** the user has typed a query, placed the text cursor mid-string, and
  presses Arrow Up or Arrow Down
- **THEN** the selection moves in the list
- **AND** the text cursor stays where it was in the prompt

#### Scenario: The selection stays in view

- **WHEN** arrowing moves the selection to a row outside the visible area
- **THEN** the list scrolls so the selected row is visible

#### Scenario: Arrowing back to the top

- **WHEN** the user arrows down into a scrolled list and then holds Arrow Up
  until the first row is selected
- **THEN** the list is scrolled to the top with that first row fully visible

#### Scenario: Arrow Up from the first result reaches the answer card

- **WHEN** root search shows an answer card and results, and the first result
  is selected
- **THEN** Arrow Up selects the card, and Arrow Up on the card leaves it
  selected
