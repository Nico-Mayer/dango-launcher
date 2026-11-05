# Spec Delta

## MODIFIED Requirements

### Requirement: View kinds

The protocol SHALL be at version 3 and SHALL support five view kinds: a list, a
grid, a detail view rendering markdown, a form, and a progress view. A progress
view SHALL declare a title and MAY declare a subtitle, a progress value from 0
to 1, a one-line detail, and steps, each with a title and a status of working,
needs input, done, failed, or queued. A progress view without a value SHALL show
an indeterminate bar, and a value outside 0 to 1 SHALL be shown clamped. Any
view kind SHALL be able to carry an action panel. A view tree at version 1 or 2
SHALL be rejected as an unsupported version.

#### Scenario: List renders items

- **WHEN** a command emits a list view containing items
- **THEN** each item's title, and its subtitle, icon, and accessories when
  present, are rendered in order

#### Scenario: Grid renders tiles

- **WHEN** a command emits a grid view containing items
- **THEN** each item is rendered as a tile with its content and title, in order,
  in the declared number of columns

#### Scenario: Detail renders markdown

- **WHEN** a command emits a detail view
- **THEN** its markdown content is rendered

#### Scenario: Form collects input

- **WHEN** a command emits a form view and the user submits it
- **THEN** the entered values are delivered to the command

#### Scenario: Progress view shows steps

- **WHEN** a command emits a progress view with a value of 0.62 and four steps
- **THEN** its bar is filled to 62 percent and its steps are listed in order
  with their status

#### Scenario: A version 1 tree

- **WHEN** a command emits a view tree at protocol version 1
- **THEN** an error is shown in place of the view

#### Scenario: A version 2 tree

- **WHEN** a command emits a view tree at protocol version 2
- **THEN** an error is shown in place of the view

### Requirement: The launcher filters lists and grids

The launcher SHALL narrow a list's or grid's items against the query as the
user types, unless the view declares that its command searches. A command
whose view does not declare it SHALL NOT receive keystrokes as query changes
and SHALL NOT be invoked again because the query changed.

#### Scenario: Typing narrows a list

- **WHEN** a list is showing and the user types
- **THEN** the visible items are narrowed without the command being invoked
  again

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
