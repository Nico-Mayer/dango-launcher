# Spec Delta

## MODIFIED Requirements

### Requirement: The launcher filters lists and grids

The launcher SHALL narrow a list's or grid's items against the query as the
user types, unless the view declares that its command searches. A command
whose view does not declare it SHALL NOT receive keystrokes as query changes
and SHALL NOT be invoked again because the query changed.

#### Scenario: Typing narrows a list

- **WHEN** a list is showing and the user types
- **THEN** the visible items are narrowed without the command being invoked
  again

### Requirement: A list or grid shows at most 50 items

A list or grid view SHALL show at most 50 items. The limit SHALL apply after
the launcher has filtered the items against the query, or, for a view whose
command searches, to the items in the order the command sent them. It SHALL
count items in the order they are drawn, across all sections; a section left
with no shown item SHALL NOT be drawn. A command MAY send more items than the
limit; the view SHALL NOT be rejected for it. Items past the limit SHALL be
reachable by typing a query that narrows the list. Selection, the action panel,
and the selected item's detail SHALL only consider the items that are shown.

#### Scenario: A long list shows the first 50 items

- **WHEN** a command emits a list of 500 items and the query is empty
- **THEN** the first 50 items, in their declared order, are shown
- **AND** the rest are not shown

#### Scenario: Filtering reaches an item past the limit

- **WHEN** a list of 500 items is shown and the user types a query that only
  the 300th item matches
- **THEN** the 300th item is shown and can be selected

#### Scenario: The limit counts across sections

- **WHEN** a list has two sections of 40 items each and the query is empty
- **THEN** all 40 items of the first section and the first 10 items of the
  second section are shown

#### Scenario: A grid is bounded too

- **WHEN** a command emits a grid of 80 tiles and the query is empty
- **THEN** the first 50 tiles are shown

#### Scenario: Moving past the last shown item

- **WHEN** a list of 500 items is shown, the 50th item is selected, and the
  user moves the selection down
- **THEN** the 50th item stays selected

#### Scenario: A command that searches sends too many items

- **WHEN** a view whose command searches is answered with 80 items for the
  query "rust"
- **THEN** the first 50 items, in the command's order, are shown

## ADDED Requirements

### Requirement: A list or grid can search through its command

A list or grid view SHALL be able to declare that its command searches. While
such a view is shown, every change to the query SHALL be delivered to the
command, which MAY emit a new view tree. The launcher SHALL NOT narrow its
items, and SHALL show them in the order the command sends. A new query SHALL
cancel the command's work for the previous one, so a tree for an older query is
never shown. Typing SHALL NOT wait on the command.

#### Scenario: The query reaches the command

- **WHEN** a view that declares its command searches is shown and the user
  types "rust"
- **THEN** the command receives the query "rust"
- **AND** the list shows the items the command emits for it, in its order

#### Scenario: An older answer is dropped

- **WHEN** the user types "ru" and then "rust", and the command answers "rust"
  before "ru"
- **THEN** the items for "rust" are shown
- **AND** the items for "ru" are never shown

#### Scenario: Items are not narrowed again

- **WHEN** the command answers the query "rust" with an item titled "Cargo"
- **THEN** "Cargo" is shown even though its title does not match "rust"

#### Scenario: The query is kept when going back

- **WHEN** the user pushes a detail view from such a list and presses Escape
- **THEN** the list shows with the query it had

### Requirement: A running action shows the launcher is busy

While an action chosen in a view or on a root item runs, the launcher SHALL
show the busy marks, following the busy-mark timing in design-system, with a
busy ticker reading "Running" followed by the action's title. The marks SHALL
go when the action finishes, whatever its outcome. Choosing a filter option and
a search that the query sends SHALL NOT show them.

#### Scenario: A slow action

- **WHEN** the user runs the action "Apply theme" and it is still running
  after the busy-mark delay
- **THEN** the busy ticker reads "Running Apply theme…"

#### Scenario: The action finishes

- **WHEN** that action finishes with a success ticker
- **THEN** the busy marks are gone and the success ticker shows

