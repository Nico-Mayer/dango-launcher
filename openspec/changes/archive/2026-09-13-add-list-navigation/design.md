## Context

See proposal.md - Why for the motivation. What matters for the approach is how
the three list surfaces are built today, and what the Bits UI `Command`
primitive already does for us (checked against `node_modules/bits-ui@2.19.2`,
`dist/bits/command/command.svelte.js`):

- Root search (`src/App.svelte`) and a pushed list view
  (`src/lib/ProtocolView.svelte`) are both `Command.Root` with
  `shouldFilter={false}`, `bind:value={selectedId}`, and rows as `Command.Item`.
- `CommandItemState.onpointermove` calls `root.setValue(...)` - hover sets the
  real selection - unless `Command.Root` gets `disablePointerSelection`.
- `src/lib/input.svelte.ts` exists only to feed that prop: it watches
  `pointermove` globally and only lets hover select once the cursor genuinely
  moved, because arrowing scrolls the list under a resting cursor and the
  browser then synthesises a pointer event. It is a workaround for hover
  selecting at all.
- Root keydown (`onkeydown` on `Command.Root`) handles Arrow Up/Down, Home, End
  and Enter, and every one of those branches calls `e.preventDefault()`. So the
  primitive already keeps arrows off the prompt's text cursor; the caret jumping
  is not the failure mode.
- `updateSelectedByItem(change)` finds the selected row's index and moves by
  `change`, with no wrap unless `loop` is set. With no selection the index is
  `-1`, so Arrow Down lands on the first row while Arrow Up lands on nothing.
  That is the "Arrow Up does nothing and the highlight is gone" case.
- The primitive re-selects the first item after items register or the selected
  item unregisters, but the launcher replaces results from the backend on every
  keystroke, and `App.svelte` papers over the gap with
  `results.find(r => r.id === selectedId) ?? results[0]`. Enter then acts on
  `results[0]` while no row carries `data-selected`.
- The action panel (`src/lib/ActionPanel.svelte`) is hand-rolled buttons with
  its own `selected` index and its own capture-phase key handler.

## Goals / Non-Goals

**Goals:**

- One rule for who owns the selection, applied identically on all three
  surfaces.
- Keep the primitive doing the work. Remove a workaround rather than add one.
- Make the marked row and the acted-on row the same piece of state, so they
  cannot disagree.

**Non-Goals:**

- Rebuilding the action panel on a Bits UI primitive.
- Any change to filtering, ranking, or the view protocol.
- Introducing new colour tokens.

## Decisions

### Turn pointer selection off outright

`disablePointerSelection={true}` on both `Command.Root`s. That is the exact
switch the primitive offers for "hover must not select", so nothing has to be
patched, and `ActionPanel` drops its `onpointermove` handler.
`src/lib/input.svelte.ts` keeps only the question of whether the pointer is
active, which the next decision uses for painting.

Alternatives rejected:

- Keep the heuristic and only restyle hover. The heuristic is guesswork about
  whether a pointer event is real, and it still lets a deliberate mouse move
  steal the keyboard's selection - which is the behaviour being removed.
- Track hover in component state and merge it with selection at render time.
  More state to keep correct for a highlight CSS can express on its own.

Click keeps working: `CommandItemState.onclick` runs the item's `onSelect` and
sets the value, independent of `disablePointerSelection`.

### Hover is a CSS-only state that cannot collide with selection

Selected row stays `data-[selected]:bg-muted`. Hover gets a weaker fill of the
same token, applied only while the row is not selected, so the two rules can
never both match:

```
class="hover:bg-muted/50 data-[selected]:bg-muted ..."
```

`.hover\:bg-muted\/50:hover` and `.data-\[selected\]\:bg-muted[data-selected]`
have identical specificity, so which one wins would come down to the order
Tailwind happens to emit. Guard it in the selector instead of trusting order:

```
class="[&:hover:not([data-selected])]:bg-muted/50 data-[selected]:bg-muted"
```

Tailwind v4's `not-*` variant (`not-data-selected:hover:bg-muted/50`) expresses
the same thing more readably; use it if it resolves on the pinned 4.3.x, and
keep the arbitrary variant otherwise. Either way the rendered rule is
`:hover:not([data-selected])`, which is what the spec requires.

Alternatives rejected:

- A new `--hover` token in `app.css`. The proposal rules out new tokens and an
  opacity modifier on `muted` already reads as "same family, quieter".
- A hover border or a left accent bar. Louder than the selected fill, which is
  the opposite of the goal.

### Hover paint is gated on pointer activity

CSS `:hover` stays true while the cursor sits still, so arrowing through the
list leaves a highlight stranded on whatever row the cursor happens to cover.
`pointerActive()` answers whether the pointer is currently driving: any key
press turns it off, a `pointermove` whose coordinates actually changed turns it
back on. The coordinate check matters because scrolling the list under a still
cursor makes the browser synthesise a `pointermove` at the same point.

The flag is published as a `data-pointer` attribute on the list container, and
the hover rule is scoped under it, so switching it off repaints without any
per-row state:

```
class="[[data-pointer]_&:hover:not([data-selected])]:bg-muted/50 data-[selected]:bg-muted"
```

Alternatives rejected:

- Track the hovered row in component state and clear it on keydown. Same effect,
  but it re-renders rows to express something CSS already knows.
- `pointer-events: none` on the list while the keyboard drives. It would also
  swallow the click that follows a deliberate mouse move.

### The selection is derived, not stored

`pickedId` holds what the keyboard or a click last chose. The selection itself
is derived from it and from the current rows:

```ts
const selectedId = $derived(
  results.some((r) => r.id === pickedId) ? pickedId : (results[0]?.id ?? ""),
);
```

so the invariant cannot be broken by any order of events: the picked row stays
selected while it exists, and the first row takes over the moment it does not.
The primitive writes back through a function binding,
`bind:value={() => selectedId, (id) => (pickedId = id)}`, which keeps `pickedId`
the only writable piece. `selectedItem` is a plain `results.find(...)`, so a row
is marked exactly when it is the row Enter acts on.

`ProtocolView.svelte` gets the same treatment against its filtered `items`.

Alternatives rejected:

- An `$effect` that repairs `selectedId` when results arrive. It works, but it
  assigns state it also reads, which the Svelte autofixer flags and which needs
  a convergence argument to review. The derived version needs none.
- Leave the `?? results[0]` fallback and rely on the primitive re-selecting the
  first item after registration. It runs `afterTick` on item registration, so
  during streaming results there is a window where nothing carries
  `data-selected`, and it does not fix Enter acting on an unmarked row.

### No wrap, and Arrow Up is fixed by the invariant, not by a key handler

With a selection always present, `updateSelectedByItem(-1)` on the first row
finds index `0`, computes `items[-1]`, gets nothing, and leaves the selection
alone - which is precisely the specified behaviour. `loop` stays unset, so Arrow
Down on the last row behaves the same way. No key handling is added or patched
in `App.svelte` or `ProtocolView.svelte`.

### The list inset sits outside the scroller

The 8px inset used to be padding on `Command.Viewport`, which scrolls with the
content: the gap was visible only at the two ends and rows ran flush to the
edges everywhere in between. Moving it to a wrapper around `Command.List`, which
keeps the border and the `py-2` while the scroller itself has no vertical
padding, makes the gap part of the frame rather than part of the content.
`Command.Viewport` keeps `px-2`, since the horizontal inset belongs to the rows.

Alternatives rejected:

- Drop the inset entirely so rows always run edge to edge. Consistent too, but
  the rounded row highlight then touches the border above it.
- Fade the top and bottom edges with a mask instead. It hides the hard cut but
  costs a gradient over a scrolling surface, and the inconsistency being
  complained about is the gap, not the cut.

### The primitive's writes are only trusted during a gesture

`#sort()` ends in `#selectFirstItem()` whenever the item set is re-sorted, and a
re-sort is scheduled every time the rows are registered again - which is every
results event. Left alone it drags the selection back to the top while results
stream in for the same query, and the old `?? results[0]` fallback hid that
rather than fixing it.

So the binding's setter takes the primitive's value only while a key press or a
click is being handled: `inUserGesture() && (pickedId = id)`. The re-sort runs on
its own, in a later task with no gesture in flight, so its reset is dropped and
the derived selection keeps the picked row. The flag is set on capture-phase
`keydown`, `pointerdown`, and `click`, and cleared with `setTimeout(..., 0)` -
not `queueMicrotask`, because Svelte delivers a component binding's write through
its effect flush, which is itself a microtask, and a microtask reset lands before
the write it is meant to cover.

A new query is the one case where the top row should win, so the prompt's setter
clears the pick: `bind:value={() => query, (q) => ((query = q), (pickedId = ""))}`.

Alternatives rejected:

- Reject any write that lands on the first row while the pick is still valid. It
  cannot tell the reset apart from Arrow Up off the second row, or from Home.
- Re-assert the pick after each results event. That is the same rule written as
  a repair after the fact, with a frame of wrong highlight in between.

### Rows do not take focus from the prompt

A `Command.Item` is not focusable, so clicking one moves focus to the body and
the prompt stops accepting input - the launcher looks alive but cannot be typed
into. The rows now prevent the default on `mousedown`, the same trick the action
panel already uses for its buttons, so the click still selects and confirms while
the cursor stays in the prompt.

### One patch: scrolling the first row into view

The primitive's `#scrollSelectedIntoView` has a blind spot. Each `Command.Item`
is wrapped in a `[data-item-wrapper]` element, so for the first row the check
`getFirstNonCommentChild(grandparent).dataset.value === item.dataset.value` is
true; it then scrolls the enclosing group's heading instead of the row and
returns. A list with no `Command.Group` has no heading, so nothing scrolls, and
arrowing back to the top leaves the first row selected one row-height above the
fold - which reads as the selection having vanished.

The patch is one effect per list, holding a `bind:ref` to `Command.List`:

```ts
$effect(() => {
  if (selectedId && selectedId === results[0]?.id && listEl) listEl.scrollTop = 0;
});
```

Only the first row is patched, because it is the only case the primitive skips,
and scrolling to `0` rather than `scrollIntoView({ block: "nearest" })` also
reveals the viewport's top padding.

Alternatives rejected:

- Wrap the rows in a `Command.Group` so the primitive has a heading to scroll
  to. It would add a group heading element to satisfy a library internal, and
  the root list has no grouping to express.
- Take over scroll-into-view for every row. The primitive handles the other
  rows correctly, including the group cases a pushed view may grow into.

### The action panel stays hand-rolled

Ctrl+K opens it while the prompt keeps the cursor and stays typeable, and a Bits
UI menu primitive moves focus into the menu, which would strand the prompt - the
same reason the panel was hand-rolled in the first place. This change only
removes its hover-selects behaviour and gives it the same hover fill as a list
row; its arrow handling already clamps at both ends and already keeps one action
selected.

## Risks / Trade-offs

- A user who hovers a row and presses Enter now gets the keyboard's row, not the
  hovered one → that is the point of the change, and the distinct hover fill
  makes which row is armed visible before Enter is pressed.
- The repair effect writes state it also reads, so a careless edit could loop →
  keep the predicate exactly "is `selectedId` still present", which is false at
  most once per results update.
- The hover fill at 50% of `muted` may be too faint on the light theme, where
  `muted` is already `hsl(240 5% 96%)` → check both themes during verification
  and adjust the modifier, not the token.
- Hover styling now relies on an arbitrary variant, which a Tailwind upgrade
  could format differently → it is two occurrences, both in row markup.

## Migration Plan

None. Frontend-only behaviour change, no stored state, no protocol version, no
data to migrate. Rollback is reverting the commit.
