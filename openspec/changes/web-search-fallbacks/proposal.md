# Proposal

## Why

When nothing in root search matches, the launcher shows "No results" and the
typed text goes nowhere. Sending that text to a web search engine is the
natural next step, and quicklinks already hold everything a search engine
needs: a URL template with `{{ query }}`, safe encoding, a browser, and a
favicon.

## What Changes

- A quicklink can be marked as a web search with a new `fallback` key in
  `quicklinks.toml`, or with a toggle in the quicklink create and edit form.
- Every typed root query lists each web search quicklink at the bottom of the
  results, under its own heading, in file order, with the typed text filled
  into `{{ query }}`. Confirming one opens the URL without asking for the
  query.
- Web search rows are not ranked, filtered, or learned from. They keep their
  place at the bottom while other results stream in above them.
- When nothing else matches, the first web search row is selected and
  "No results" is not shown. "No results" still shows when no quicklink is a
  web search.
- On startup, when `quicklinks.toml` does not exist, Dango creates it with one
  DuckDuckGo web search entry. Deleting that entry turns it off. No search
  engine is built into the code.

## Non-goals

- A keyword prefix that picks one engine (`g foo` for Google). Possible later
  improvement, not part of this change.
- Search suggestions or results fetched from an engine.
- Showing web search rows on the empty query or inside pushed views.
- Ranking or reordering web search rows by use.
- Any other default engine than DuckDuckGo, or a list of presets to pick from.

## Platforms

Windows and macOS. The change is platform-neutral: it reuses the quicklink
opener and browser handling that both platforms already have.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `quicklinks`: a quicklink can be a web search, set in the form or the file,
  offered under every typed query with the query filled in.
- `root-search`: web search rows sit at the bottom of a typed query's results
  under a heading, unranked, and take the place of "No results"; the rule that
  a typed query never shows section headings gains this exception.
- `config-files`: `quicklinks.toml` gains the `fallback` key, and Dango creates
  the file with a DuckDuckGo entry when it is missing at startup.
- `interface-copy`: "No results" shows only when no web search is offered
  either.

## Impact

- `src/extensions/snippets/` (quicklink record, form, root provider, actions,
  file seeding).
- `src/search/` and `src/ranking/` (a new result source that is never ranked
  and goes last).
- `src/ui/` state machine and snapshot (bottom section, selection, empty
  state), and `src/app/` (no frecency or learned input for web search rows).
- `quicklinks.toml` schema and settings reference.
- No view protocol or extension manifest change.
