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

### Requirement: Providers have a time budget

Each root items provider SHALL be queried asynchronously with a short time
budget. A provider exceeding its budget SHALL NOT delay results from others; its
results SHALL be merged in when they arrive.

#### Scenario: A slow provider does not block the list

- **WHEN** one provider is slow and the others answer at once
- **THEN** the fast results are displayed without waiting for the slow one
- **AND** the slow provider's results are merged in when they arrive

#### Scenario: A provider never answers

- **WHEN** a provider does not return at all
- **THEN** results from every other provider are still displayed
- **AND** the pipeline does not leak the abandoned work

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

- **WHEN** the user types ten characters in 300 milliseconds
- **THEN** at most one query is in flight at any moment

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
- **THEN** only the bounded top slice is sent to the frontend

### Requirement: A result carries its extension's tint

A root search result SHALL be presented with the tint declared by the extension
that contributed it, so results from different extensions can be told apart
without reading their titles. The tint SHALL be applied as a filled background
behind the result's icon, with the icon drawn in a colour that stays legible
against it in both the light and the dark theme.

The tint SHALL apply only where the icon is one of the launcher's own named
icons. A result whose icon is an image the extension produced, such as an
application's own icon, SHALL be presented without a tinted background, and so
SHALL a result with no icon at all.

The tint SHALL NOT change which results appear, their order, or the time taken
to produce them.

#### Scenario: Two built-in commands are told apart

- **WHEN** a query returns commands from two extensions that declare different tints
- **THEN** each command's icon sits on its own extension's tint

#### Scenario: Commands of one extension share its tint

- **WHEN** a query returns several commands from the same extension
- **THEN** every one of them shows the same tint

#### Scenario: An application result keeps its own icon

- **WHEN** a query returns an application whose own icon was extracted
- **THEN** that icon is shown with no tinted background behind it

#### Scenario: An untinted extension is unchanged

- **WHEN** a query returns a result from an extension that declares no tint
- **THEN** the result is presented as it was before tints existed

#### Scenario: A dynamic result is tinted like its extension's commands

- **WHEN** a query returns an item from a root items provider whose extension declares a tint, and the item's icon is a named icon
- **THEN** it shows the same tint as that extension's commands

#### Scenario: Tinting does not cost search time

- **WHEN** the index holds 2000 candidates and the user types a character
- **THEN** ranked results still appear without visible delay

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
