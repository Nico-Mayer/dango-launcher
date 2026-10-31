## ADDED Requirements

### Requirement: The launcher filters lists and grids

The launcher SHALL narrow a list's or grid's items against the query as the
user types. A command SHALL NOT receive keystrokes as query changes and SHALL
NOT be invoked again because the query changed. A view SHALL NOT be able to
hand filtering to its command.

#### Scenario: Typing narrows a list

- **WHEN** a list is showing and the user types
- **THEN** the visible items are narrowed without the command being invoked
  again

### Requirement: A Shift or Alt shortcut names a letter key

An action shortcut that includes Shift or Alt SHALL name a letter key, A to Z.
Dango SHALL refuse a view tree in which any action declares Shift or Alt with
any other key, and SHALL show an error in place of the view, as for any other
invalid tree. A shortcut without Shift or Alt MAY name any key.

On macOS, Option with a letter and without Cmd or Ctrl types a composed
character, so a shortcut of Alt and a letter alone SHALL NOT be expected to
run there.

#### Scenario: A Shift shortcut on a letter runs

- **WHEN** a view declares an action with the shortcut Ctrl+Shift+P and the
  user presses Ctrl+Shift+P
- **THEN** that action runs

#### Scenario: A Shift shortcut on a digit is refused

- **WHEN** a command emits a view with an action whose shortcut is Shift+1
- **THEN** the view is rejected and an error is shown in its place

#### Scenario: An Alt shortcut on a symbol is refused

- **WHEN** a command emits a view with an action whose shortcut is Alt+/
- **THEN** the view is rejected and an error is shown in its place

#### Scenario: An Alt shortcut on a letter runs on Windows

- **WHEN** a view on Windows declares an action with the shortcut Alt+P and
  the user presses Alt+P
- **THEN** that action runs

#### Scenario: An Alt shortcut on a letter alone does not run on macOS

- **WHEN** a view on macOS declares an action with the shortcut Alt+P and the
  user presses Option+P
- **THEN** the action does not run

### Requirement: A list or grid shows at most 50 items

A list or grid view SHALL show at most 50 items. The limit SHALL apply after
the launcher has filtered the items against the query, and SHALL count items in
the order they are drawn, across all sections; a section left with no shown item
SHALL NOT be drawn. A command MAY send more items than the limit; the view SHALL
NOT be rejected for it. Items past the limit SHALL be reachable by typing a
query that narrows the list. Selection, the action panel, and the selected
item's detail SHALL only consider the items that are shown.

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

## REMOVED Requirements

### Requirement: Filtering ownership is declared

**Reason**: No effect carries a view's query to its command, so a list that
declared command filtering never narrowed. No built-in used it.

**Migration**: None. The launcher always filters; see "The launcher filters
lists and grids". Bring command filtering back with a delivering effect when a
command needs it.

### Requirement: A list or grid shows a bounded number of items

**Reason**: Its scenario "A command-filtered list is bounded too" and its
sentence on command filtering no longer apply. A delta cannot drop a scenario
from a modified requirement, so the requirement is replaced.

**Migration**: Replaced by "A list or grid shows at most 50 items", the same
rule without command filtering.
