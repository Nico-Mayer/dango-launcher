## RENAMED Requirements

- FROM: `### Requirement: Providers have a time budget`
- TO: `### Requirement: Slow providers stream in`

## MODIFIED Requirements

### Requirement: Slow providers stream in

Each root items provider SHALL be queried asynchronously. Results that are
ready SHALL be shown at once, without waiting for slower providers, and a slow
provider's results SHALL be merged in when they arrive. A provider that has not
answered after a few seconds SHALL be dropped from that query.

#### Scenario: A slow provider does not block the list

- **WHEN** one provider is slow and the others answer at once
- **THEN** the fast results are displayed without waiting for the slow one
- **AND** the slow provider's results are merged in when they arrive

#### Scenario: A provider never answers

- **WHEN** a provider does not return at all
- **THEN** results from every other provider are still displayed
- **AND** after a few seconds the query stops waiting for that provider

#### Scenario: A provider fails

- **WHEN** a provider returns an error
- **THEN** the remaining results are displayed and the failure is recorded where a developer can read it

### Requirement: Cancel and restart on new input

Each keystroke SHALL cancel the in-flight query and start a new one. Results
from a cancelled query SHALL NOT be displayed. The pipeline SHALL NOT debounce
input.

#### Scenario: Stale results are discarded

- **WHEN** the user types a second character before the first query completes
- **THEN** only results matching the two-character query are displayed

#### Scenario: Fast typing does not queue work

- **WHEN** the user types quickly
- **THEN** only results for the final text are shown

### Requirement: Ranking combines match quality and frecency

Results SHALL be ranked by fuzzy match quality against title, keywords, and
alias, combined with a frecency score derived from how often and how recently
the item was launched, and with what the user has previously chosen for the
typed text. The order of precedence SHALL be: an exact alias match first, then
an item the user previously chose for exactly the typed text, then every other
match by combined score. Within the last group, an item the user previously
chose for a longer text that starts with the typed text SHALL receive a boost
that grows with how often and how recently that choice was made.

#### Scenario: Frequently used items rise

- **WHEN** two items match a query equally well and one has been launched many times recently
- **THEN** the frequently launched item ranks higher

#### Scenario: Recency decays

- **WHEN** two items have been launched the same number of times and one was last launched months ago
- **THEN** the recently launched item ranks higher

#### Scenario: Alias wins

- **WHEN** the user types a string that exactly matches an item's alias
- **THEN** that item is ranked first, ahead of any learned choice

#### Scenario: A learned exact input wins over text

- **WHEN** the user previously confirmed an item after typing exactly `cal`, and now types `cal` again
- **THEN** that item is ranked first among all non-alias results, even if another item has a better fuzzy match and higher frecency

#### Scenario: A learned prefix boosts

- **WHEN** the user previously confirmed Calendar after typing `cal`, and now types `c`
- **THEN** Calendar ranks above an item with a slightly better fuzzy match that was never chosen for a text starting with `c`

#### Scenario: Typing past a learned input releases it

- **WHEN** the user previously confirmed an item after typing `cal`, and now types `calc`
- **THEN** that earlier choice has no effect on the ranking of `calc`

#### Scenario: Subsequence matching

- **WHEN** the user types characters that appear in order but not adjacently in an item's title
- **THEN** the item matches
- **AND** the matched characters are marked so they can be highlighted

### Requirement: Empty query behaviour

With an empty query the root view SHALL show a bounded list of the most
frecently used items rather than everything available, and SHALL present that
list in labelled sections when there is something to point out.

A provider MAY mark an item as pinned, such as a missing permission warning or
an available update. With an empty query, pinned items SHALL come first, under
a section headed "Needs attention", in provider order, not in the order the
providers answered. Being pinned SHALL have no effect on a typed query: there
the item is matched and ranked like any other.

The section of launched items SHALL be headed "Suggestions" and SHALL hold at
most six items, chosen as the items with the highest frecency among those that
have been launched at least once. The last section SHALL be headed "Everything
else" and SHALL hold the remainder of the bounded list, highest frecency first,
then by title. An item SHALL appear in exactly one section.

When there is no pinned item and no item has been launched yet, the list SHALL
be shown flat, with no section headings. A non-empty query SHALL never show
section headings.

#### Scenario: Empty query shows recents

- **WHEN** the launcher opens with an empty query and items have been used before
- **THEN** the most frecently used items are shown

#### Scenario: Empty query on first run

- **WHEN** the launcher opens with an empty query, nothing has been used yet, and no item is pinned
- **THEN** a bounded default list is shown rather than an empty view
- **AND** no section heading is shown

#### Scenario: Pinned items need attention first

- **WHEN** the launcher opens with an empty query and a provider has a pinned item, such as a missing permission
- **THEN** a section headed "Needs attention" is shown first, holding the pinned items
- **AND** the rest of the list follows under "Suggestions" and "Everything else"

#### Scenario: Pinned items keep provider order

- **WHEN** two providers each have a pinned item and the second provider answers first
- **THEN** the first provider's pinned item is still shown above the second's

#### Scenario: A pinned item in a typed query

- **WHEN** the user types a query that matches a pinned item
- **THEN** the pinned item is ranked like any other match, not placed first

#### Scenario: Suggestions sit above everything else

- **WHEN** the launcher opens with an empty query and at least one item has been launched
- **THEN** a section headed "Suggestions" is shown, holding the launched items in frecency order
- **AND** a section headed "Everything else" follows, holding the rest of the bounded list

#### Scenario: The suggestions section is bounded

- **WHEN** more than six distinct items have been launched
- **THEN** the "Suggestions" section holds exactly six items
- **AND** the seventh and later launched items appear under "Everything else" ahead of never-launched items

#### Scenario: An item is never shown twice

- **WHEN** an item is shown under "Suggestions"
- **THEN** it does not also appear under "Everything else"

#### Scenario: The top suggestion is selected

- **WHEN** the launcher opens with an empty query, no item is pinned, and a "Suggestions" section is shown
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
- **THEN** the list is scrolled so the first section's heading and the first row are both fully visible

#### Scenario: Splitting does not cost search time

- **WHEN** the index holds 2000 candidates and the query is empty
- **THEN** the sectioned list appears without visible delay

### Requirement: Search stays responsive as the index grows

Ranking and merging SHALL keep up with typing for an index of 2000 candidates,
and the result list SHALL be bounded so rendering cost does not grow with index
size.

#### Scenario: Large index

- **WHEN** the index holds 2000 candidates and the user types a character
- **THEN** ranked results appear without visible delay

#### Scenario: Broad query

- **WHEN** a query matches more than a thousand candidates
- **THEN** only the bounded top slice is shown

### Requirement: Learned inputs decay and yield to correction

A learned input SHALL be the lowercased, trimmed text the user had typed at
the moment of confirming, paired with the confirmed item and a count of how
often that pair was confirmed. Its weight SHALL decay with the same half life
frecency uses, so a pair that stops being used fades. When the user confirms
a different item for the same text, the new pair SHALL compete on count and
recency rather than replacing the old one, so one slip does not undo an
established habit and a repeated correction does win. Learned inputs SHALL
have no effect on the empty query.

#### Scenario: Case and whitespace do not split a habit

- **WHEN** the user confirms an item after typing `Cal` and later after typing ` cal`
- **THEN** both count toward the same learned input `cal`

#### Scenario: A repeated correction wins

- **WHEN** the user confirmed Calendar for `cal` once, then confirmed Calculator for `cal` three times
- **THEN** typing `cal` ranks Calculator first

#### Scenario: A single slip does not win

- **WHEN** the user confirmed Calendar for `cal` ten times this week and Calculator once
- **THEN** typing `cal` still ranks Calendar first

#### Scenario: An unused pair fades

- **WHEN** a pair was confirmed a few times six months ago and a competing pair for the same text was confirmed once this week
- **THEN** the recent pair ranks its item first

#### Scenario: The empty query is unchanged

- **WHEN** the query is empty
- **THEN** suggestions and ordering follow frecency alone

#### Scenario: Learning does not slow search

- **WHEN** 2000 learned pairs exist and the user types a two-letter query over 2000 candidates
- **THEN** ranked results still appear without visible delay
