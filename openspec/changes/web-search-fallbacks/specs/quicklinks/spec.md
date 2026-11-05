# Spec Delta

## ADDED Requirements

### Requirement: A quicklink can be a web search

A quicklink SHALL be a web search when it is marked as one, either with the
"Use for web search" toggle in its create and edit form or with
`fallback = true` in `quicklinks.toml`. A web search's URL SHALL use `query`
and SHALL ask for nothing else; `clipboard` and `selection` MAY fill
themselves. A quicklink not marked SHALL NOT be a web search.

#### Scenario: Marking a quicklink in the form

- **WHEN** the user turns on "Use for web search" in a quicklink's form and saves
- **THEN** the quicklink is stored as a web search
- **AND** its entry in `quicklinks.toml` has `fallback = true`

#### Scenario: The edit form shows the stored state

- **WHEN** the user edits a quicklink that is a web search
- **THEN** the "Use for web search" toggle is on

#### Scenario: Unmarking a quicklink

- **WHEN** the user turns off "Use for web search" and saves
- **THEN** the quicklink is no longer a web search
- **AND** its entry in `quicklinks.toml` has no `fallback` key

#### Scenario: Marking a quicklink in the file

- **WHEN** `quicklinks.toml` holds an entry with `fallback = true` and a URL
  that uses `query`
- **THEN** that quicklink is a web search

#### Scenario: A quicklink without the key

- **WHEN** a quicklink entry has no `fallback` key
- **THEN** it is not a web search

#### Scenario: A web search without a query is refused in the form

- **WHEN** the user saves a quicklink with "Use for web search" on and a URL
  that does not use `query`
- **THEN** nothing is stored
- **AND** the form says "For web search, the URL must ask only for {{ query }}."

#### Scenario: A web search asking for more is refused in the form

- **WHEN** the user saves a quicklink with "Use for web search" on and a URL
  that uses `query` and another argument
- **THEN** nothing is stored
- **AND** the form says "For web search, the URL must ask only for {{ query }}."

#### Scenario: An unusable web search in the file

- **WHEN** `quicklinks.toml` holds an entry with `fallback = true` and a URL
  that does not use `query`, or also asks for another argument
- **THEN** it is offered as an ordinary quicklink and not as a web search

### Requirement: A web search is offered for every typed query

Each web search SHALL be offered as a row for every non-empty root query,
titled "Search", its name, "for", and the query in quotes. The row SHALL carry
the quicklink's icon. The quicklink SHALL also stay findable by name like any
other quicklink. When the quicklinks extension is disabled, no web search SHALL
be offered.

#### Scenario: A web search row

- **WHEN** a web search quicklink is named DuckDuckGo and the user types `rust traits`
- **THEN** root search shows a row titled `Search DuckDuckGo for “rust traits”`
- **AND** the row shows DuckDuckGo's favicon when it is available

#### Scenario: The quicklink is still found by name

- **WHEN** the user types part of a web search quicklink's name
- **THEN** the quicklink appears among the ranked results as usual
- **AND** its web search row also appears for that query

#### Scenario: The quicklinks extension is disabled

- **WHEN** the quicklinks extension is disabled and the user types a query
- **THEN** no web search row is shown

### Requirement: Confirming a web search opens it with the typed query

Confirming a web search row SHALL open the quicklink with the typed query as
its `query` value, without showing a form, the same way a quicklink with a
submitted query opens: encoded, in the quicklink's browser or the default
browser. The row SHALL also offer copying the rendered URL.

#### Scenario: Opening a web search

- **WHEN** the user confirms the row `Search DuckDuckGo for “rust traits”`
- **THEN** the launcher closes and DuckDuckGo's URL opens with `rust traits`
  as the query
- **AND** no form asks for the query

#### Scenario: Reserved characters stay in the query

- **WHEN** the user confirms a web search row for the query `a&b?c`
- **THEN** the characters are encoded in the opened URL
- **AND** the URL's own structure is unchanged

#### Scenario: A web search with a browser

- **WHEN** the user confirms a web search row whose quicklink names an
  installed browser
- **THEN** the URL opens in that browser

#### Scenario: A web search using the clipboard

- **WHEN** a web search's template uses `query` and `clipboard`
- **THEN** confirming its row opens the URL with the typed query and the
  clipboard text filled in, without a form

#### Scenario: Copying a web search URL

- **WHEN** the user chooses "Copy" in a web search row's action panel
- **THEN** the rendered URL with the typed query is on the clipboard
- **AND** the launcher closes
