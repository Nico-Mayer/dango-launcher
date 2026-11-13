# root-search Specification

## Purpose

The pipeline that turns each keystroke in the root prompt into a ranked list of
results, gathering candidates from every enabled contributor without ever
letting a slow one make the launcher feel slow.

## Requirements

### Requirement: Results come from multiple contributors

Root search SHALL gather candidates from every command in the registry and from
every enabled extension's root items provider, merge them, and present one
ranked list.

#### Scenario: Commands and dynamic items appear together

- **WHEN** a query matches both a registered command and an item from a root items provider
- **THEN** both appear in the same ranked list

#### Scenario: Disabled extensions contribute nothing

- **WHEN** an extension is disabled
- **THEN** neither its commands nor its root items appear for any query

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

### Requirement: Launching an item updates its frecency

Confirming a root result SHALL record the event so that the item's future
ranking improves, and SHALL record the text the user had typed when they
confirmed it, so the same text ranks that item higher next time. Both records
SHALL survive a restart. Any successful root action SHALL count, not only the
primary one. An action inside a pushed view SHALL update frecency but SHALL
NOT record a typed text, because none was typed for that item. A launch from
a global hotkey SHALL update neither: a chord says nothing about what the user
types to find an item, so pressing it SHALL NOT change search ranking.

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
- **THEN** its frecency is unchanged
- **AND** no typed text is recorded for it

#### Scenario: A command chord leaves ranking alone

- **WHEN** the user presses the chord bound to Maximize twenty times and then types `ze`
- **THEN** Maximize ranks no higher for `ze` than it did before those presses

### Requirement: Empty query behaviour

With an empty query the root view SHALL start closed: it SHALL show no
results, and the launcher SHALL be the SearchBar pill alone. When
`launcher.open-list` is `true`, the root view SHALL instead start with the
empty-query list open, the same as after Arrow Down (see launcher-shell). After the first
typed character, the pill SHALL stay alone until the first result for the
query arrives, and SHALL NOT show the empty-query list meanwhile. When the
query becomes empty again, the root view SHALL close again, or, when
`launcher.open-list` is `true`, return to the open list.

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
show section headings other than "Search the web" (see "Web searches close a
typed query's results").

#### Scenario: The launcher opens as the pill

- **WHEN** the launcher opens with an empty query, no item is pinned, and
  `launcher.open-list` is absent
- **THEN** only the SearchBar pill is shown, with no results and no ActionBar

#### Scenario: The setting starts with the list open

- **WHEN** `launcher.open-list` is `true` and the launcher opens after a reset
- **THEN** the empty-query list is open with the same sections as after
  Arrow Down
- **AND** the first row is selected

#### Scenario: The first character shows results

- **WHEN** the launcher shows the pill alone and the user types one character
- **THEN** the pill stays alone until the first result for that character
  arrives
- **AND** the card then opens with one ranked list for that query, with no
  heading other than "Search the web" below it

#### Scenario: Clearing the query brings the sections back

- **WHEN** the user deletes the last character of a query, leaving it empty,
  no item is pinned, and `launcher.open-list` is absent
- **THEN** the card closes and only the pill is shown
- **AND** Arrow Down then shows the "Suggestions" and "Everything else"
  sections again

#### Scenario: Clearing the query returns to the open list

- **WHEN** `launcher.open-list` is `true` and the user deletes the last
  character of a query, leaving it empty
- **THEN** the empty-query list is open with its sections, and the first row
  is selected

#### Scenario: Typing removes the sections

- **WHEN** the user has opened the empty-query list and types one character
- **THEN** the list shows one ranked list, with no heading other than
  "Search the web" below it

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

### Requirement: Confirming a result acts on it

Root search SHALL act on the selected result when the user confirms it. A
command result SHALL be invoked; any other result SHALL have its primary action
performed. The result's remaining actions SHALL be reachable from its action
panel.

#### Scenario: Confirming a command result

- **WHEN** the user confirms a selected command result
- **THEN** that command is invoked

#### Scenario: Confirming a root item

- **WHEN** the user confirms a selected result contributed by a root items provider
- **THEN** that result's primary action is performed

#### Scenario: Choosing a secondary action

- **WHEN** the user opens a result's action panel and chooses an action other than the primary one
- **THEN** that action is performed instead of the primary one

#### Scenario: Confirming with no selection

- **WHEN** the user confirms while no result is selected, because the list is empty
- **THEN** nothing is invoked
- **AND** the launcher stays open

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

### Requirement: Matching folds case and accents and prefers word starts

Fuzzy matching of the query against a title, keyword or alias SHALL ignore
letter case and SHALL treat an unaccented Latin letter in the query as
matching its accented forms in the text. Among texts that contain the query
as a subsequence, a match that lands on the start of the text or on word
starts SHALL score higher than one that lands mid-word, and a contiguous match
SHALL score higher than a scattered one. The positions reported for
highlighting SHALL index characters, not bytes, so an accented title
highlights correctly.

#### Scenario: Case is ignored

- **WHEN** the user types `FIRE` and an item is titled "Firefox"
- **THEN** the item matches

#### Scenario: Accents are folded

- **WHEN** the user types `cafe` and an item is titled "Café"
- **THEN** the item matches
- **AND** the highlighted positions mark the four characters of the title

#### Scenario: A word start beats a mid-word hit

- **WHEN** the user types `ze`, and "Zed" and "Maximize" have equal frecency and no learned input
- **THEN** "Zed" ranks above "Maximize"

#### Scenario: A contiguous match beats a scattered one

- **WHEN** the user types `fire`, and "Firefox" and "Far Infrared Emitter" have equal frecency and no learned input
- **THEN** "Firefox" ranks above "Far Infrared Emitter"

### Requirement: Each result names its extension

Each root search result SHALL show, beside its title, the name of the
extension that owns it, as that extension's manifest gives it. This SHALL hold
for commands and for root items alike. A pinned result SHALL show its own
status line there instead. No other result SHALL show a path, a content
preview, or any other text in that place.

#### Scenario: An application names its extension

- **WHEN** root search lists an application
- **THEN** the text beside its title reads "Applications"
- **AND** the application's path is not shown in the row

#### Scenario: A command names its extension

- **WHEN** root search lists Clipboard History
- **THEN** the text beside its title reads "Clipboard History"

#### Scenario: A root item from a provider names its extension

- **WHEN** root search lists a Steam game
- **THEN** the text beside its title reads "Games"

#### Scenario: A snippet shows no preview

- **WHEN** root search lists a snippet
- **THEN** the text beside its title reads "Snippets"
- **AND** no part of the snippet's body is shown in the row

#### Scenario: A pinned result keeps its status line

- **WHEN** a missing permission is listed as a pinned result
- **THEN** the text beside its title is that permission's status line, not "Permissions"

### Requirement: A result shows its hotkey

A command or application result whose global hotkey is registered SHALL show
that hotkey on its right as a key chord, using the platform's names for the
keys. A result without a working hotkey SHALL show none.

#### Scenario: A command's hotkey on Windows

- **WHEN** "Clipboard History" has the hotkey "ctrl+alt+v" on Windows and the
  user types "clip"
- **THEN** its row shows "Ctrl Alt V" as a key chord

#### Scenario: A command's hotkey on macOS

- **WHEN** "Clipboard History" has the hotkey "ctrl+alt+v" on macOS and the user
  types "clip"
- **THEN** its row shows "⌃⌥V" as a key chord

#### Scenario: An application's hotkey

- **WHEN** an application has a registered hotkey and the user types its name
- **THEN** its row shows that hotkey

#### Scenario: A hotkey that could not be registered

- **WHEN** a command's configured hotkey was refused by the operating system
- **THEN** its row shows no key chord

### Requirement: Web searches close a typed query's results

For a non-empty query, web search rows (see quicklinks) SHALL be listed after
every other result, under the heading "Search the web", in the order of their
entries in `quicklinks.toml`. They SHALL be shown only once every provider has
answered the query. They SHALL NOT be matched against the query, ranked, or
reordered by use. They SHALL count toward the result limit, but other results
SHALL be cut first. The empty query SHALL show none.

#### Scenario: Web searches sit below the results

- **WHEN** the user types `fire` and Firefox and two web searches match
- **THEN** Firefox is listed first and selected
- **AND** the two web search rows follow under "Search the web", in file order

#### Scenario: Web searches wait for every provider

- **WHEN** the user types a query and a provider has not answered yet
- **THEN** no web search row is shown
- **AND** the web search rows appear below the results once every provider
  has answered

#### Scenario: Enter before every provider answered

- **WHEN** the user types `fire` and confirms before every provider has answered
- **THEN** no web search is opened

#### Scenario: Results are cut before web searches

- **WHEN** a query matches more results than the limit allows
- **THEN** every web search row is still shown at the bottom
- **AND** the list holds no more rows than the limit

#### Scenario: The empty query shows no web search

- **WHEN** the user opens the empty-query list
- **THEN** no web search row is shown

### Requirement: Web searches stand in for no results

When a typed query has no result other than web search rows, the first web
search row SHALL be selected and "No results" SHALL NOT be shown.

#### Scenario: Nothing else matches

- **WHEN** the user types `zzqx`, no other result matches, and DuckDuckGo is a
  web search
- **THEN** once every provider has answered, the row
  `Search DuckDuckGo for “zzqx”` is shown and selected
- **AND** "No results" is not shown

#### Scenario: No web search configured

- **WHEN** no quicklink is a web search and the user types a query nothing matches
- **THEN** "No results" is shown, as before

### Requirement: A web search teaches nothing

Confirming a web search row, or choosing any of its actions, SHALL NOT update
frecency or record the typed text for any item.

#### Scenario: Searching the web leaves ranking alone

- **WHEN** the user types `rust`, confirms a web search row, and later types `rust` again
- **THEN** results rank exactly as they did before the web search
