## MODIFIED Requirements

### Requirement: The SearchBar holds the query and shows where the user is

The SearchBar SHALL hold the query for a list or grid, with a placeholder the
view sets. When a view has been pushed, it SHALL show a back control before
the query; activating it SHALL go back one view, as Escape does. While the view
is loading, it SHALL show a thin bar along its bottom edge in which a short
segment in the accent role sweeps from one end to the other, again and again,
at a steady pace. When the view declares a filter dropdown, the SearchBar SHALL
show it at its right end. For a detail or form view, the SearchBar SHALL show
the command's title in place of the query.

#### Scenario: Root shows no back control

- **WHEN** the launcher is at root
- **THEN** the SearchBar shows no back control

#### Scenario: A pushed view shows the back control

- **WHEN** a command has pushed a list
- **THEN** the SearchBar shows a back control before the query

#### Scenario: Clicking back

- **WHEN** the user clicks the back control
- **THEN** the previous view is shown with its query and selection intact

#### Scenario: A loading view

- **WHEN** the view is marked loading
- **THEN** the loading bar is shown under the SearchBar with its segment
  sweeping along it

#### Scenario: Loading ends

- **WHEN** the view stops being marked loading, or the user leaves it
- **THEN** the loading bar and its motion are gone in the same frame

#### Scenario: A form shows its title

- **WHEN** a command pushes a form
- **THEN** the SearchBar shows that command's title and accepts no typing

### Requirement: Nothing animates

Dango SHALL NOT animate its interface, except for the loading bar's sweep.
Showing the launcher, moving the selection, hovering, opening and closing the
action panel, showing and removing a toast, changing results, and replacing a
view SHALL each change in one frame. Busy states SHALL be shown by the loading
bar and by static marks next to text. How fast the launcher appears is in
launcher-shell, and how a streamed view keeps its selection is in
view-protocol.

#### Scenario: Activation on Windows

- **WHEN** the launcher is shown on Windows, first or again
- **THEN** it appears without an entrance effect

#### Scenario: Activation on macOS

- **WHEN** the launcher is shown on macOS, first or again
- **THEN** it appears without an entrance effect

#### Scenario: A streamed view stays stable

- **WHEN** a command replaces one view many times a second
- **THEN** each replacement shows in one frame, with no transition between the
  old and the new content
- **AND** the loading bar's sweep continues without restarting

#### Scenario: Busy without motion

- **WHEN** work is in progress
- **THEN** the loading bar's segment is the only thing that moves
- **AND** the busy toast's dot stays still

#### Scenario: Idle without motion

- **WHEN** no view is loading
- **THEN** no animation runs on any surface
