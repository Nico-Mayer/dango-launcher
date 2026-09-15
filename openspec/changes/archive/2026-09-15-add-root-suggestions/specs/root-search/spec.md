## MODIFIED Requirements

### Requirement: Empty query behaviour

With an empty query the root view SHALL show a bounded list of the most
frecently used items rather than everything available, and SHALL present that
list in two labelled sections when there is something to suggest.

The first section SHALL be headed "Suggestions" and SHALL hold at most six
items, chosen as the items with the highest frecency among those that have been
launched at least once. The second section SHALL be headed "Everything else"
and SHALL hold the remainder of the bounded list, ordered as the empty-query
list is ordered today: highest frecency first, then by title. An item SHALL
appear in exactly one section.

When no item has been launched yet, the list SHALL be shown flat, with no
section headings. A non-empty query SHALL never show section headings.

The split SHALL be decided where launches are recorded, so the frontend never
labels an item a suggestion on its own account.

#### Scenario: Empty query shows recents

- **WHEN** the launcher opens with an empty query and items have been used before
- **THEN** the most frecently used items are shown

#### Scenario: Empty query on first run

- **WHEN** the launcher opens with an empty query and nothing has been used yet
- **THEN** a bounded default list is shown rather than an empty view
- **AND** no section heading is shown

#### Scenario: Suggestions sit above everything else

- **WHEN** the launcher opens with an empty query and at least one item has been launched
- **THEN** a section headed "Suggestions" is shown first, holding the launched items in frecency order
- **AND** a section headed "Everything else" follows, holding the rest of the bounded list

#### Scenario: The suggestions section is bounded

- **WHEN** more than six distinct items have been launched
- **THEN** the "Suggestions" section holds exactly six items
- **AND** the seventh and later launched items appear under "Everything else" ahead of never-launched items

#### Scenario: An item is never shown twice

- **WHEN** an item is shown under "Suggestions"
- **THEN** it does not also appear under "Everything else"

#### Scenario: The top suggestion is selected

- **WHEN** the launcher opens with an empty query and a "Suggestions" section is shown
- **THEN** the first item of that section is the selected row

#### Scenario: Typing removes the sections

- **WHEN** the user types one character into the empty prompt
- **THEN** the list shows one ranked list with no section heading

#### Scenario: Clearing the query brings the sections back

- **WHEN** the user deletes the last character of a query, leaving it empty, and items have been launched before
- **THEN** the "Suggestions" and "Everything else" sections are shown again

#### Scenario: A launch moves an item into suggestions

- **WHEN** the user launches an item that was under "Everything else" and then reopens the launcher with an empty query
- **THEN** that item appears under "Suggestions"

#### Scenario: Arrowing back to the top shows the heading

- **WHEN** the user arrows down into the scrolled empty-query list and then holds Arrow Up until the first row is selected
- **THEN** the list is scrolled so the "Suggestions" heading and the first row are both fully visible

#### Scenario: Splitting does not cost search time

- **WHEN** the index holds 2000 candidates and the query is empty
- **THEN** the sectioned list is ready within 30 milliseconds
