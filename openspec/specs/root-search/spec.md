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

Each root items provider SHALL be queried asynchronously with a budget of 50
milliseconds. A provider exceeding its budget SHALL NOT delay results from
others; its results SHALL be merged in when they arrive.

#### Scenario: A slow provider does not block the list

- **WHEN** one provider takes 400 milliseconds and others answer in 5 milliseconds
- **THEN** the fast results are displayed within the budget
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
the item was launched. An exact alias match SHALL outrank everything else.

#### Scenario: Frequently used items rise

- **WHEN** two items match a query equally well and one has been launched many times recently
- **THEN** the frequently launched item ranks higher

#### Scenario: Recency decays

- **WHEN** two items have been launched the same number of times and one was last launched months ago
- **THEN** the recently launched item ranks higher

#### Scenario: Alias wins

- **WHEN** the user types a string that exactly matches an item's alias
- **THEN** that item is ranked first

#### Scenario: Subsequence matching

- **WHEN** the user types characters that appear in order but not adjacently in an item's title
- **THEN** the item matches
- **AND** the matched characters are marked so the frontend can highlight them

### Requirement: Launching an item updates its frecency

Invoking a result SHALL record the event so that the item's future ranking
improves. The record SHALL survive a restart.

#### Scenario: Ranking improves after use

- **WHEN** the user launches a low-ranked result for a query and later types the same query
- **THEN** that result ranks higher than before

#### Scenario: Frecency persists

- **WHEN** the user launches an item and restarts the application
- **THEN** the item's improved ranking is retained

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

### Requirement: Search stays responsive as the index grows

Ranking and merging SHALL produce results within 30 milliseconds for an index of
2000 candidates, and the result list SHALL be bounded so rendering cost does not
grow with index size.

#### Scenario: Large index

- **WHEN** the index holds 2000 candidates and the user types a character
- **THEN** ranked results are ready within 30 milliseconds

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
- **THEN** ranked results are still ready within 30 milliseconds

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
