## MODIFIED Requirements

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
- **AND** the matched characters are marked so the frontend can highlight them

### Requirement: Launching an item updates its frecency

Confirming a root result SHALL record the event so that the item's future
ranking improves, and SHALL record the text the user had typed when they
confirmed it, so the same text ranks that item higher next time. Both records
SHALL survive a restart. Any successful root action SHALL count, not only the
primary one. A launch from a global hotkey and an action inside a pushed view
SHALL update frecency as today but SHALL NOT record a typed text, because none
was typed for that item.

#### Scenario: Ranking improves after use

- **WHEN** the user launches a low-ranked result for a query and later types the same query
- **THEN** that result ranks first among non-alias results

#### Scenario: Frecency persists

- **WHEN** the user launches an item and restarts the application
- **THEN** the item's improved ranking is retained

#### Scenario: A secondary action teaches too

- **WHEN** the user types `cal`, opens the action panel, and chooses a non-primary action on Calendar that succeeds
- **THEN** `cal` is recorded for Calendar the same as a launch would have

#### Scenario: A hotkey launch teaches nothing

- **WHEN** the user opens an application through its global hotkey
- **THEN** its frecency rises
- **AND** no typed text is recorded for it

## ADDED Requirements

### Requirement: Learned inputs decay and yield to correction

A learned input SHALL be the lowercased, trimmed text the user had typed at
the moment of confirming, paired with the confirmed item and a count of how
often that pair was confirmed. Its weight SHALL decay with the same half life
frecency uses, so a pair that stops being used fades. When the user confirms
a different item for the same text, the new pair SHALL compete on count and
recency rather than replacing the old one, so one slip does not undo an
established habit and a repeated correction does win. Learned inputs SHALL
have no effect on the empty query. Recording and loading them SHALL not add
work to the search path: ranking SHALL read them from memory.

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
- **THEN** suggestions and ordering follow frecency alone, as before

#### Scenario: Learning does not slow search

- **WHEN** 2000 learned pairs exist and the user types a two-letter query over 2000 candidates
- **THEN** ranking completes within the existing 30 millisecond budget
