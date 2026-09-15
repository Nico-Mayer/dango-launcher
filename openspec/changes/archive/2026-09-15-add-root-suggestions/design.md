## Context

See proposal.md - Why. What shapes the approach:

- The ranker already has a dedicated empty-query path, `rank_empty_query`,
  that scores every candidate by frecency and sorts highest first, then by
  title. On a first run every score is zero and the sort falls back to title
  order. The result is one bounded `Vec<Candidate>` of up to `RESULT_LIMIT`
  (50) items.
- Frecency lives in memory in `FrecencyTable`, is read on the search path, and
  is written on launch. An item never launched scores exactly zero; a launched
  item decays towards zero but never reaches it.
- `SearchPipeline` streams `SearchResults` snapshots; `lib.rs` maps each
  `Candidate` to a `ResultItem` and emits it on the `dango://results` event.
  That event is the internal contract between Rust and the webview. It is not
  the versioned view protocol.
- The root list in `App.svelte` is the Bits UI `Command` primitive with
  filtering off, rendering one flat `{#each results}` of `Command.Item`. No
  `Command.Group` is used anywhere yet.
- `App.svelte` carries a workaround: the primitive's scroll-into-view stops as
  soon as the selected row is inside the list's edges, so arrowing back to the
  first row left it just under the top edge with the inset above it hidden. The
  workaround sets `scrollTop = 0` for the first row.
- The empty query runs at warm-up and on every reset to root, so its cost is
  paid on every activation.

## Goals / Non-Goals

**Goals:**

- The empty prompt reads as "your usual picks, then the rest", at a glance.
- The backend owns the split. The frontend stays a renderer.
- No measurable cost on the empty-query path.
- The primitive's own grouping does the layout and the scroll behaviour.

**Non-Goals:**

- Beyond the proposal's list: no change to the view protocol's `ListView`, so a
  pushed list still has no sections.
- No third section and no per-extension grouping.
- No change to `FrecencyTable` scoring.

## Decisions

### The backend marks suggestions; the frontend groups

`Candidate` gains `suggested: bool`, false by default and set only by the
ranker. `rank_empty_query` sorts as today, then flags the first items whose
frecency score is above zero, up to `SUGGESTION_LIMIT = 6`. Everything after
the flagged run is unmarked. `ResultItem` in `lib.rs` copies the flag, and the
frontend splits the ordered results into the marked prefix and the rest.

Rejected: letting the frontend show the first N rows as suggestions when the
query is empty. It cannot tell a launched item from a never-launched one, so on
a first run it would label six alphabetical items "Suggestions", which is
false. The proposal's rule that the split is decided where launches are
recorded exists for this reason.

Rejected: a `suggested_count: usize` on `SearchResults`. It says the same thing
as a prefix of flagged items, but the `Ranker` trait returns a plain
`Vec<Candidate>`, so a count would need a new return type or a second call. A
flag on the candidate rides the existing shape and the existing
`ResultItem::new` mapping.

Rejected: a general `section: Option<String>` label on each item, so the
backend could later add "Favourites" or per-extension sections. Two fixed
sections are the ask and nothing planned needs more. A string label would also
move interface copy into Rust, while today every string the root list shows
lives in the Svelte file.

### Which items qualify

An item is a suggestion if its frecency score is above zero, so it has been
launched at least once. There is no minimum threshold above zero. With a
30-day half-life an item launched once a year ago still scores about 0.0002,
and it still counts, because the alternative is a magic cut-off that hides an
item the user did use. Six slots is the cap; when more items qualify, the six
highest win and the rest stay in frecency order under "Everything else", ahead
of never-launched items, exactly where they are today.

Six is the author's guess at "a handful that fits above the fold at the
default window height", the same order as Raycast's section. It is one
constant next to `FRECENCY_WEIGHT` and moves as easily.

Rejected: a threshold such as "launched at least twice" or "used in the last 30
days". Both hide a real launch behind a rule the user cannot see, and both
turn a first launch into "why is it not up there yet".

### Sections only when there is something to suggest

When no candidate has a score above zero, the ranker flags nothing, and the
frontend renders the flat list it renders today, with no heading at all.
"Everything else" without a "Suggestions" above it would be a heading that
explains nothing. The rule falls out of the flag: headings appear if and only
if at least one result is marked.

### The list is the primitive's own groups

The root list stays on `Command.Root` with `shouldFilter={false}`. The two
sections are two `Command.Group` blocks, each with a `Command.GroupHeading` and
a `Command.GroupItems` wrapping the same `Command.Item` and `ResultRow` markup
used today. When nothing is marked, the existing ungrouped `{#each}` renders
as it does now, so the query path and the first-run path do not change shape.

Nothing is patched. Arrow keys cross group boundaries on their own. The
primitive's scroll-into-view has a branch meant to scroll a group's heading in
when the first item of the group is selected, but in Bits UI 2.19 it compares
the item's value against the `GroupItems` wrapper, which has none, so it never
fires and every row gets a plain nearest-edge scroll. Two things cover that
without touching the primitive. The `scrollTop = 0` workaround already in
`App.svelte` handles the first row, and so the "Suggestions" heading, since the
first row is the first suggestion. A `scroll-margin-top` on the first row of
each group, the height of a heading, makes the primitive's own nearest-edge
scroll leave room for the "Everything else" heading when arrowing up lands on
that row. Both are CSS or a one-line effect, not a fork of the primitive.

Rejected: patching the primitive's scroll branch. A scroll margin gets the same
result from the browser's own scroll-into-view, and survives upgrades.

Rejected: hand-rolling two headings as plain `<div>` rows between items in the
flat `{#each}`. It would work visually, but a heading the primitive does not
know about gets no `role="group"` or `aria-labelledby`, and there would be no
first-of-group row to hang the scroll margin on. Using the primitive's own
grouping is also the project rule.

Rejected: `Command.Separator` with no headings. A rule between two runs of rows
does not say what the runs are.

### Heading style

The heading is small, muted, and left-aligned with the row content: the
`text-muted-foreground` token, `text-xs`, medium weight, with the row's
horizontal padding and a little more space above than below so it attaches to
the rows under it. This matches the footer and subtitle treatment already in
`App.svelte` and `ResultRow.svelte`, so nothing new is introduced to the token
set.

### User-facing text

Two new strings, both headings in the root list:

- "Suggestions"
- "Everything else"

Both are sentence case, one or two plain words, contain no identifier, and
name what is in the section. "Suggestions" is the word Raycast users already
know for the same thing. "Everything else" was chosen over "All commands"
(false, since the list holds applications, quicklinks, and snippets too),
"Apps and commands" (the prompt placeholder's phrasing, but equally incomplete),
and "All results" (there is no query, so nothing has resulted). It says exactly
what the section is: the bounded list minus the suggestions. Both are quoted in
the spec's scenarios in their final form.

No failure, empty state, or status line changes. "No results for" is unaffected
because it only shows for a non-empty query.

## Risks / Trade-offs

- [Six suggestions push the first "Everything else" row below the fold at the
  default window height] → Accepted. The point of the section is that the
  fold falls between the two, and Arrow Down or typing gets past it in one
  keystroke. If six proves too many in daily use, the constant moves.
- [A stale item lingers in suggestions because nothing else has been launched]
  → Accepted by design. It is still the most used thing the user has, and it
  leaves as soon as anything else is launched.
- [The primitive's group markup changes the DOM between the grouped and the
  flat list, so the `data-pointer` hover styling or the `Command.Item` classes
  behave differently inside `Command.GroupItems`] → Both are attribute
  selectors on the item and its ancestor, not on siblings, so the wrapping
  should not matter. Worth checking during verification on both webviews.
- [Every empty-query result now carries one more boolean over the event
  channel] → Fifty booleans. Not measurable.
- [The flag leaks into the ranked-query path] → The ranker only sets it in
  `rank_empty_query`; the non-empty path never touches it, and a test pins
  that a non-empty query marks nothing.

## Migration Plan

Nothing to migrate. Frecency records are unchanged; the first activation after
the change shows sections if anything was ever launched. Rollback is removing
the flag and the two groups.
