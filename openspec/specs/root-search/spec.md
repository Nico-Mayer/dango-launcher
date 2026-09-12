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
frecently used items rather than everything available.

#### Scenario: Empty query shows recents

- **WHEN** the launcher opens with an empty query and items have been used before
- **THEN** the most frecently used items are shown

#### Scenario: Empty query on first run

- **WHEN** the launcher opens with an empty query and nothing has been used yet
- **THEN** a bounded default list is shown rather than an empty view

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
