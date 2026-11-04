## MODIFIED Requirements

### Requirement: Empty query behaviour

With an empty query the root view SHALL start closed: it SHALL show no
results, and the launcher SHALL be the SearchBar pill alone. After the first
typed character, the pill SHALL stay alone until the first result for the
query arrives, and SHALL NOT show the empty-query list meanwhile. When the
query becomes empty again, the root view SHALL close again.

A provider MAY mark an item as pinned, such as a missing permission warning or
an available update. While any item is pinned, the closed root view SHALL
show the card with only those items, under a section headed "Needs
attention", in provider order, not in the order the providers answered. Being
pinned SHALL have no effect on a typed query: there the item is matched and
ranked like any other.

The user MAY open the empty-query list with Arrow Down, as list-navigation
describes. The open list SHALL be a bounded list of the most frecently used
items rather than everything available, presented in labelled sections when
there is something to point out: pinned items first under "Needs attention",
then a section headed "Suggestions" holding at most six items, chosen as the
items with the highest frecency among those that have been launched at least
once, and last a section headed "Everything else" holding the remainder of
the bounded list, highest frecency first, then by title. An item SHALL appear
in exactly one section.

When there is no pinned item and no item has been launched yet, the open list
SHALL be shown flat, with no section headings. A non-empty query SHALL never
show section headings.

#### Scenario: The launcher opens as the pill

- **WHEN** the launcher opens with an empty query and no item is pinned
- **THEN** only the SearchBar pill is shown, with no results and no ActionBar

#### Scenario: The first character shows results

- **WHEN** the launcher shows the pill alone and the user types one character
- **THEN** the pill stays alone until the first result for that character
  arrives
- **AND** the card then opens with one ranked list for that query and no
  section heading

#### Scenario: Clearing the query brings the sections back

- **WHEN** the user deletes the last character of a query, leaving it empty,
  and no item is pinned
- **THEN** the card closes and only the pill is shown
- **AND** Arrow Down then shows the "Suggestions" and "Everything else"
  sections again

#### Scenario: Typing removes the sections

- **WHEN** the user has opened the empty-query list and types one character
- **THEN** the list shows one ranked list with no section heading

#### Scenario: The top suggestion is selected

- **WHEN** the user opens the empty-query list with Arrow Down, no item is
  pinned, and a "Suggestions" section is shown
- **THEN** the first item of that section is the selected row

#### Scenario: Pinned items open the card on their own

- **WHEN** the launcher opens with an empty query and a provider has a pinned
  item, such as a missing permission
- **THEN** the card is shown with only a section headed "Needs attention",
  holding the pinned items, and the first of them is selected

#### Scenario: Pinned items keep provider order

- **WHEN** two providers each have a pinned item and the second provider answers first
- **THEN** the first provider's pinned item is still shown above the second's

#### Scenario: A pinned item in a typed query

- **WHEN** the user types a query that matches a pinned item
- **THEN** the pinned item is ranked like any other match, not placed first

#### Scenario: Empty query shows recents

- **WHEN** the user opens the empty-query list and items have been used before
- **THEN** the most frecently used items are shown

#### Scenario: Empty query on first run

- **WHEN** the user opens the empty-query list, nothing has been used yet, and
  no item is pinned
- **THEN** a bounded default list is shown rather than an empty view
- **AND** no section heading is shown

#### Scenario: Suggestions sit above everything else

- **WHEN** the user opens the empty-query list and at least one item has been launched
- **THEN** a section headed "Suggestions" is shown, holding the launched items in frecency order
- **AND** a section headed "Everything else" follows, holding the rest of the bounded list

#### Scenario: Pinned items need attention first

- **WHEN** the user opens the empty-query list while an item is pinned
- **THEN** "Needs attention" comes first, followed by "Suggestions" and
  "Everything else"

#### Scenario: The suggestions section is bounded

- **WHEN** more than six distinct items have been launched
- **THEN** the "Suggestions" section holds exactly six items
- **AND** the seventh and later launched items appear under "Everything else" ahead of never-launched items

#### Scenario: An item is never shown twice

- **WHEN** an item is shown under "Suggestions"
- **THEN** it does not also appear under "Everything else"

#### Scenario: A launch moves an item into suggestions

- **WHEN** the user launches an item that was under "Everything else" and then
  reopens the launcher and opens the empty-query list
- **THEN** that item appears under "Suggestions"

#### Scenario: Arrowing back to the top shows the heading

- **WHEN** the user arrows down into the scrolled empty-query list and then holds Arrow Up until the first row is selected
- **THEN** the list is scrolled so the first section's heading and the first row are both fully visible

#### Scenario: Splitting does not cost search time

- **WHEN** the index holds 2000 candidates and the user opens the empty-query list
- **THEN** the sectioned list appears without visible delay
