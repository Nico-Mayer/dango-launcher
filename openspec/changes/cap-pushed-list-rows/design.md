## Context

A pushed list or grid reaches the screen through one path in the launcher state
machine: `group` in `src/ui/state.rs` narrows the items by the query (when the
launcher filters) and groups them by section. `view_sections` and
`grid_sections` call it, and everything else reads from them: `entries`, the
selection step, the action panel, the detail pane, and the snapshot the
renderer draws. Root search is bounded separately by `RESULT_LIMIT` (50) in
`src/app/mod.rs`, before results enter the state machine.

The renderer builds every row it receives in a `ScrollView` (see the ListView
note in AGENTS.md), and sizes the window by counting rows.

## Goals / Non-Goals

**Goals:**

- One place enforces the limit for every pushed list and grid, so selection,
  panel, detail, and drawing cannot disagree about which items exist.
- Root search and pushed views share one number.

**Non-Goals:**

- No change to the view tree, the manifest, or the protocol version.
- No renderer change beyond what follows from receiving fewer rows.

## Decisions

### The limit is applied in `group`, after filtering

`group` stops once 50 items have been taken, counting in draw order: loose
items first, then each declared section in order. Sections left empty are
dropped, as today.

Alternatives rejected:

- **Cap in the renderer** (`src/slint_shell/render/list.rs`). The state machine
  would still let the selection and the action panel reach item 300, which the
  renderer no longer shows. AGENTS.md keeps interaction rules out of the
  renderer.
- **Cap in each extension** (Clipboard History sends 50). The launcher filters
  locally, so a capped history could never be narrowed to entry 300. It would
  also leave every other extension unbounded.
- **Cap before filtering.** Same problem: items past 50 could never be found by
  typing.

### One constant for root and views

The limit becomes one constant in `src/ui`, and `src/app` uses it for
`RESULT_LIMIT` instead of its own `50`. Root search and pushed views then
cannot drift apart.

### The view protocol stays at version 2

The cap is a rule about what the launcher shows, not about what a command may
send: a tree with 500 items is still valid and is not rejected. No field is
added, removed, or reinterpreted, so the version does not change. The new
requirement in `view-protocol` records the limit so a future third-party
extension can rely on it: a command that needs more than 50 items reachable
without typing must use sections and filtering, or several views.

## Risks / Trade-offs

- [An item past 50 is not reachable without typing] → Same rule as root search;
  Clipboard History is filtered by typing anyway. No "show more" row, by the
  proposal's non-goals.
- [A command-filtered list that sends 120 items loses 70 of them] → No built-in
  command filters itself today. The requirement states the limit, so an
  extension author knows to narrow its own results.
- [The remembered selection can point past the limit after the query changes]
  → `pick_entry` already falls back to the first shown entry when the picked id
  is not among the entries.

## Migration Plan

None. No stored data or config changes. Rollback is reverting the commit.
