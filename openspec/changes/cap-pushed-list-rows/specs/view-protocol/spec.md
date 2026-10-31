## ADDED Requirements

### Requirement: A list or grid shows a bounded number of items

A list or grid view SHALL show at most 50 items. The limit SHALL apply after
the launcher has filtered the items against the query, and SHALL count items in
the order they are drawn, across all sections; a section left with no shown item
SHALL NOT be drawn. The limit SHALL apply whether the launcher or the command
filters. A command MAY send more items than the limit; the view SHALL NOT be
rejected for it. Items past the limit SHALL be reachable by typing a query that
narrows the list. Selection, the action panel, and the selected item's detail
SHALL only consider the items that are shown.

#### Scenario: A long list shows the first 50 items

- **WHEN** a command emits a list of 500 items and the query is empty
- **THEN** the first 50 items, in their declared order, are shown
- **AND** the rest are not shown

#### Scenario: Filtering reaches an item past the limit

- **WHEN** a list of 500 items is shown, its launcher filters, and the user
  types a query that only the 300th item matches
- **THEN** the 300th item is shown and can be selected

#### Scenario: The limit counts across sections

- **WHEN** a list has two sections of 40 items each and the query is empty
- **THEN** all 40 items of the first section and the first 10 items of the
  second section are shown

#### Scenario: A command-filtered list is bounded too

- **WHEN** a list declares that its command filters and emits 120 items
- **THEN** the first 50 items are shown

#### Scenario: A grid is bounded too

- **WHEN** a command emits a grid of 80 tiles and the query is empty
- **THEN** the first 50 tiles are shown

#### Scenario: Moving past the last shown item

- **WHEN** a list of 500 items is shown, the 50th item is selected, and the
  user moves the selection down
- **THEN** the 50th item stays selected
