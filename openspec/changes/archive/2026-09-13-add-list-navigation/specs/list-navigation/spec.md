## Purpose

How every list of rows in the launcher is selected and navigated: what the
keyboard owns, what the pointer owns, and how a selected row is told apart from
a hovered one. It applies to root search, a list view a command pushed, and the
action panel.

## ADDED Requirements

### Requirement: A non-empty list always has exactly one selected row

Whenever a list shows at least one row, exactly one of those rows SHALL be
selected and SHALL be marked as selected on screen. An empty list SHALL have no
selection.

The selection SHALL be the row a confirmation acts on, so what the user sees
highlighted and what Enter acts on can never disagree.

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

### Requirement: The prompt stays typeable while the list holds the selection

The prompt SHALL hold the keyboard focus the whole time the launcher is open,
except while a surface that owns the keyboard is up, such as the action panel.
Typing SHALL always reach the prompt, without the user having to click it or
arrow back to it, and having a selected row SHALL NOT take typing away from it.

#### Scenario: Typing after arrowing

- **WHEN** the user arrows down several rows and then types a character
- **THEN** the character is inserted into the prompt and the query narrows

#### Scenario: Typing after clicking a row is not needed elsewhere

- **WHEN** the user clicks a row's surrounding area without invoking it and then
  types
- **THEN** the character is inserted into the prompt

### Requirement: The pointer never changes the selection by hovering

Moving the pointer over a row SHALL NOT change which row is selected, however
long it rests there and whether or not the list moves under a resting pointer.
Only the keyboard and a click SHALL move the selection.

#### Scenario: Hovering a different row

- **WHEN** a row is selected and the user moves the pointer over another row
- **THEN** the selected row does not change

#### Scenario: The list scrolls under a resting pointer

- **WHEN** the user arrows through the list while the pointer rests over the
  list area
- **THEN** the selection follows the arrow keys only, and rows sliding under the
  pointer change nothing

#### Scenario: Moving the pointer off the list

- **WHEN** the user moves the pointer away from the list
- **THEN** the selected row is unchanged and stays marked as selected

#### Scenario: Clicking a row acts on that row

- **WHEN** the user clicks a row
- **THEN** that row becomes the selected row
- **AND** it is confirmed, exactly as if it had been selected with the keyboard
  and confirmed

### Requirement: Hover and selection are visually distinct

A hovered row SHALL carry a highlight that is clearly weaker than the selected
row's, so both can be shown at once and the selected row still reads as the one
Enter will act on. The selected row's appearance SHALL NOT change when the
pointer is over it, and it SHALL NOT be possible for a hovered row to look as
prominent as the selected row.

The hover highlight SHALL disappear as soon as the user presses a key, and SHALL
return only once the pointer moves again, so a cursor left resting over the list
paints nothing while the keyboard drives the selection.

#### Scenario: Hovering an unselected row

- **WHEN** the pointer rests over a row that is not selected
- **THEN** that row shows the weaker hover highlight
- **AND** the selected row keeps its own, stronger highlight

#### Scenario: Hovering the selected row

- **WHEN** the pointer rests over the selected row
- **THEN** the row's appearance is unchanged

#### Scenario: The pointer leaves

- **WHEN** the pointer moves off a hovered row
- **THEN** the hover highlight disappears and nothing else changes

#### Scenario: A key press clears a resting hover

- **WHEN** the pointer rests over a row and the user presses a key
- **THEN** the hover highlight disappears
- **AND** the selected row is the only row highlighted

#### Scenario: Hover returns when the pointer moves again

- **WHEN** a key press has cleared the hover highlight and the user then moves
  the pointer
- **THEN** the row under the pointer shows the hover highlight again

#### Scenario: Arrowing scrolls the list under a resting pointer

- **WHEN** arrowing scrolls rows under a cursor that has not moved
- **THEN** no row shows a hover highlight

### Requirement: The list keeps a constant inset

The gap between a list's top and bottom edges and its rows SHALL be the same at
every scroll position, so scrolling does not make the list appear to change its
padding.

#### Scenario: Scrolled to the top

- **WHEN** the list is scrolled to the top
- **THEN** there is a gap between the list's top edge and the first row

#### Scenario: Scrolled part way

- **WHEN** the list is scrolled so that rows are cut off at its top and bottom
  edges
- **THEN** the gap at both edges is the same as when the list is at rest

### Requirement: The rules hold on every list surface

Root search, a list view pushed by a command, and the action panel SHALL all
follow these selection, keyboard, and hover rules.

#### Scenario: A pushed list view

- **WHEN** a command pushes a list view
- **THEN** it opens with a row selected, arrows move that selection without
  touching its prompt, and hover only highlights

#### Scenario: The action panel

- **WHEN** the action panel is open
- **THEN** one action is always selected, arrows move that selection, and
  hovering an action only highlights it
