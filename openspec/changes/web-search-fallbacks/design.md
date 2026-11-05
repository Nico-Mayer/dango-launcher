# Design

## Context

See proposal.md for why. Requirements are in the delta specs under `specs/`.

Current state that shapes the approach:

- Quicklinks live in `src/extensions/snippets/` and share `Records` with
  snippets. A record is name, URL template, optional browser
  (`QuicklinkEntry` in `store.rs`). `quicklinks.toml` is a shared file: Dango
  writes it with `toml_edit` and keeps comments, order, unknown keys.
- A missing records file loads as an empty set (`store.rs`, `load`). Nothing
  creates it until the first save from the launcher.
- Opening already does what a web search needs: `Template::render_url` encodes
  every value, `open_in` falls back to the default browser, favicons come from
  `FaviconCache` by host.
- Root search merges candidates from commands and `RootProvider`s and sends
  them through `Ranker::rank`. The calculator answer is the precedent for a
  row that is not ranked: `Source::Answer` is split off before ranking and
  placed first, and its value travels in the item id (`ID_PREFIX`).
- The UI state machine keeps `picked` only when the keyboard or a click chose
  a row. Without it, the first row in `root_order` is selected.
- `Core::record_use` already skips the calculator so answers do not fill the
  frecency and learned-input tables.

## Goals / Non-Goals

**Goals:**

- Web search rows built from the quicklink records, with no second store and
  no second opener.
- Rows that never move: always last, never ranked, never cut before a ranked
  result.

**Non-Goals:**

- Changing the `Extension` trait, the view protocol, or the extension
  manifest.
- A new UI component.

## Decisions

### 1. A web search is a quicklink with `fallback = true`

`QuicklinkEntry` gains `fallback: bool` (absent = false, `skip_serializing_if`
false, so unmarking removes the key). `Records::save` takes it the same way it
takes `browser` today. The schema and settings reference pick it up from the
struct.

Rejected: a `[web-search.engines.<id>]` table in `config.toml`. It would
duplicate URL templates, browser choice, favicons, and encoding. Also,
`config.toml` is user-owned, so the launcher could never add an engine from a
form.

Rejected: a built-in DuckDuckGo in code with an `enabled` setting. The user
wants no engine in core. A file entry is turned off by deleting it, the same
way as any other quicklink.

### 2. The quicklinks provider emits the web search rows

`SnippetProvider::items` already receives the query. For a non-empty query
and kind `Quicklink`, it also emits one candidate per record with `fallback`
whose template uses `query`, in file order. These candidates have
`source: Source::WebSearch`, the record's icon, and the title
`Search <name> for “<query>”`.

Rejected: a separate `web-search` extension. It would need its own access to
the quicklink records and icons, and its own enabled flag, which would differ
from "quicklinks disabled = no web search".

### 3. `Source::WebSearch` is split off before ranking, like `Source::Answer`

`DangoRanker::rank` partitions out `WebSearch` candidates and ranks the rest
with `limit - web_searches.len()`, then appends the web searches. This puts
them last on every streamed emit, and other results are cut first. On the
empty query the provider emits none, so the ranker needs no special case for
that.

Rejected: give them the lowest score inside ranking. Frecency and learned
inputs could still lift them, and the 50-row cut would drop them first.

### 4. The typed query travels in the item id

The candidate id is `web:<record-id>:<query>`. Record ids are kebab-case and
contain no `:`, so splitting at the first `:` after the prefix is unambiguous.
`perform_action` strips the prefix, loads the record, and adds the query as
the `query` argument before the existing argument check. So "asks only for the
other arguments" and "copy" reuse the current `ACTION_INSERT` and `ACTION_COPY`
paths unchanged.

Rejected: passing the root query to `Extension::perform_action`. That changes
a trait every extension implements, only for this one caller. The calculator
already uses the id this way.

### 5. UI state knows web search rows by a flag on `ResultItem`

`ResultItem` gains `web_search: bool`, set in `Core::result_item` from the
source (as `answer` is today). `root_order` and `root_screen` put these rows
last, in their own `Section` headed "Search the web". The "No results" empty
text is built only when no row is left at all, which already holds once the
web search rows are in `results`.

The selection rule needs no new state: `picked` is only set by the user, so
an unpicked selection follows the first row in `root_order` when a result
arrives above the web search rows. A web search the user picked keeps its id
(the id holds the query, which does not change within one query).

### 6. Web searches teach nothing

`Core::record_use` returns early for item ids with the `web:` prefix of the
quicklinks extension, next to the existing calculator check. The prefix is a
`pub` helper in the quicklinks module, so the check does not repeat the
string.

### 7. Seeding the file at startup

When the quicklinks extension is built and enabled, and `quicklinks.toml` is
missing, `Records` creates it through the same write path as a save: the
`#:schema` header from `UserFile::Quicklinks`, then

```toml
[duckduckgo]
name = "DuckDuckGo"
url = "https://duckduckgo.com/?q={{ query }}"
fallback = true
```

It runs once per start in `register_snippets` (`src/app/startup.rs`), after
`Records::open` and before the file watch starts, and goes through the
records' own-write marker like any save, so it does not trigger a reload.

Rejected: seed when the file is first written from the launcher. A user who
never makes a quicklink would never get a web search.

### 8. The form toggle

The quicklink form gets a `FieldKind::Toggle` field after the browser
dropdown. It uses the existing `ui/atoms/checkbox.slint` through the form view
(`ui/views/form.slint`). Rows use the existing list row and section heading in
`ui/views/list.slint`. No new design-system part.

Save checks, before storing, that a quicklink marked as a web search has a
template that uses `query` (`Template::uses`), and refuses it with a message
otherwise.

### Contracts

No change to the extension manifest or view protocol. `Source`, `Candidate`,
and `ResultItem` are internal types.

### Interface text

All new strings, checked against `openspec/specs/interface-copy`:

| Where | String |
| --- | --- |
| Root section heading | "Search the web" |
| Web search row title | `Search <name> for “<query>”`, e.g. `Search DuckDuckGo for “rust traits”` |
| Row primary action (ActionBar) | "Open" |
| Row second action | "Copy" |
| Form toggle label | "Use for web search" |
| Form validation | "Add {{ query }} to the URL to use it for web search." |

"Open" and "Copy" match the existing quicklink actions and the verb rules.
The row title is not a command title, so it uses sentence case with the
engine's name kept as written. `{{ query }}` in the validation message is the
text the user types in the URL field, not an internal identifier.

## Risks / Trade-offs

- [The query is part of the item id, so a long query makes a long id] → The
  id is never stored, because `record_use` skips it, and is only compared
  within one query.
- [`list-navigation` says a selected item stays selected when rows are
  replaced. The web search rule gives up an unpicked selection.] → The code
  already follows the first row when nothing is picked. The new
  `root-search` requirement states the web search case explicitly.
- [Deleting `quicklinks.toml` brings DuckDuckGo back] → Accepted. Deleting the
  entry is the way to turn it off, and it stays off.
- [A web search row appears for every query, also when good results exist] →
  The user chose this. Rows sit last and are never selected over a result.
- [Every keystroke clones the web search rows with a new title] → Usually one
  to three records. Building them costs nothing next to the ranker.
