## ADDED Requirements

### Requirement: Arrow Down opens the empty-query list

At root with an empty prompt and the list closed (see root-search), Arrow
Down SHALL open the empty-query list. From the bare pill it SHALL select the
first row. From the "Needs attention" card it SHALL keep that card's rows at
the top of the open list and move the selection down one row, as it would in
an open list. Arrow Up SHALL NOT open the list.

#### Scenario: Arrow Down on the bare pill

- **WHEN** the launcher shows the bare pill and the user presses Arrow Down
- **THEN** the card opens with the empty-query list and its first row is
  selected

#### Scenario: Arrow Down on the "Needs attention" card

- **WHEN** the launcher shows the "Needs attention" card with one pinned item
  selected and the user presses Arrow Down
- **THEN** the full empty-query list is shown and the row after the pinned
  item is selected

#### Scenario: Arrow Up on the bare pill

- **WHEN** the launcher shows the bare pill and the user presses Arrow Up
- **THEN** nothing changes

#### Scenario: Typing after opening the list

- **WHEN** the user has opened the empty-query list and types a character
- **THEN** the list shows the ranked results for that character, with the
  first row selected
