## Why

With nothing typed, the root list already puts the most frecently used items
first, but it shows them as one flat run of rows that looks no different from a
search result. The user cannot tell where "what I usually reach for" ends and
"everything Dango knows about" begins, so the empty prompt reads as a long
unsorted list rather than a set of ready picks. Raycast draws that line with a
"Suggestions" section above the rest, and that is the shape the author wants
when the launcher opens.

Milestone: a follow-up to M1, which shipped root search. It is polish in the
spirit of M7 and opens no new milestone.

Platforms: macOS and Windows, one code path. The change is a ranking rule in
Rust and a grouped list in the webview, neither of which touches the OS. Linux
stays out of scope.

## What Changes

- With an empty query, the root list is split into two labelled sections. The
  first, "Suggestions", holds the handful of items the user launches most, by
  frecency. The second, "Everything else", holds the rest of the bounded list
  in the order it has today.
- An item appears in one section only. A suggested item is not repeated below.
- The sections appear only when there is something to suggest. On a first run,
  or with a fresh frecency store, the list is flat and unlabelled as it is now.
- Typing any character removes the sections. A search result list is one ranked
  list, exactly as before.
- The first row of the list, and so the initial selection, is the top
  suggestion when there is one.
- The backend decides which items are suggestions, since only it knows what has
  been launched. The frontend groups what it is told.

## Non-goals

- Suggestions while a query is typed. Raycast does not do this either, and the
  ranked list already folds frecency in.
- Pinned or favourite items, or any way to remove an item from the suggestions
  by hand. Frecency decides, as it does for ranking.
- Per-extension or per-kind sections, such as "Applications" or "Commands".
  Two sections are the ask.
- Sections in a list view a command pushed. The view protocol does not move.
- A configurable suggestion count. It is a constant in the ranker.
- Any change to how frecency is scored or recorded.

## Capabilities

### New Capabilities

None. This changes how an existing capability presents itself.

### Modified Capabilities

- `root-search`: the "Empty query behaviour" requirement changes from "show a
  bounded frecent list" to "show a labelled suggestions section above the rest,
  and only when there is something to suggest".

## Impact

- `src-tauri/src/ranking/mod.rs`: the empty-query path marks the top frecent
  items as suggestions.
- `src-tauri/src/search/mod.rs`: the candidate carries the mark.
- `src-tauri/src/lib.rs`: the result item sent to the frontend carries it too.
  This is the internal results event between Rust and the webview, not the
  versioned view protocol, so no `protocolVersion` change.
- `src/App.svelte` and `src/lib/types.ts`: the root list renders two groups
  with headings when the results say so, using the `Command` primitive's own
  grouping. No new component.
- Two new user-facing strings, "Suggestions" and "Everything else", quoted in
  the design and in the spec.
- No new dependency, no migration, no manifest change.
