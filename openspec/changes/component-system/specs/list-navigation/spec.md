## MODIFIED Requirements

### Requirement: The prompt stays typeable while the list holds the selection

The prompt SHALL hold the keyboard focus the whole time the launcher is open,
except while a surface that owns the keyboard is up, such as the action panel.
Typing SHALL always reach the prompt, without the user having to click it or
arrow back to it, and having a selected row SHALL NOT take typing away from it.
While the action panel is open, typing SHALL reach the panel's filter input
instead, and SHALL reach the prompt again once the panel closes.

#### Scenario: Typing after arrowing

- **WHEN** the user arrows down several rows and then types a character
- **THEN** the character is inserted into the prompt and the query narrows

#### Scenario: Typing after clicking a row is not needed elsewhere

- **WHEN** the user clicks a row's surrounding area without invoking it and then
  types
- **THEN** the character is inserted into the prompt

#### Scenario: Typing with the action panel open

- **WHEN** the action panel is open and the user types a character
- **THEN** the character goes to the panel's filter input and the prompt's query
  is unchanged

#### Scenario: Typing after the panel closes

- **WHEN** the user closes the action panel with Escape and types a character
- **THEN** the character is inserted into the prompt

### Requirement: The rules hold on every list surface

Root search, a list view pushed by a command, a grid view, and the action panel
SHALL all follow these selection, keyboard, and hover rules. In a grid, the
rules for rows apply to tiles.

#### Scenario: A pushed list view

- **WHEN** a command pushes a list view
- **THEN** it opens with a row selected, arrows move that selection without
  touching its prompt, and hover only highlights

#### Scenario: A grid view

- **WHEN** a command pushes a grid view
- **THEN** it opens with the first tile selected, arrows move that selection
  without touching its prompt, and hover only highlights

#### Scenario: The action panel

- **WHEN** the action panel is open
- **THEN** one action is always selected, arrows move that selection, and
  hovering an action only highlights it

## ADDED Requirements

### Requirement: A grid moves in four directions

In a grid, Arrow Left and Arrow Right SHALL move the selection to the previous
and next tile in reading order, and Arrow Up and Arrow Down SHALL move it to the
tile in the same column one row up or down. When Arrow Down finds no tile in the
same column of the next row, it SHALL select that row's last tile. The
selection SHALL NOT wrap past the first or last tile. While a grid is shown,
Arrow Left and Arrow Right SHALL NOT move the prompt's text cursor.

#### Scenario: Moving right

- **WHEN** the second tile of a five-column grid is selected and the user
  presses Arrow Right
- **THEN** the third tile is selected

#### Scenario: Moving right at a row's end

- **WHEN** the fifth tile of a five-column grid is selected and the user presses
  Arrow Right
- **THEN** the sixth tile, the first of the next row, is selected

#### Scenario: Moving down

- **WHEN** the second tile of a five-column grid is selected and the user
  presses Arrow Down
- **THEN** the seventh tile is selected

#### Scenario: Moving down into a short last row

- **WHEN** a five-column grid holds seven tiles, the fifth is selected, and the
  user presses Arrow Down
- **THEN** the seventh tile is selected

#### Scenario: The last tile stays

- **WHEN** the last tile is selected and the user presses Arrow Right or Arrow
  Down
- **THEN** the last tile stays selected

#### Scenario: The text cursor stays

- **WHEN** a grid is shown, the user has typed a query, and presses Arrow Left
- **THEN** the selection moves and the text cursor stays where it was
