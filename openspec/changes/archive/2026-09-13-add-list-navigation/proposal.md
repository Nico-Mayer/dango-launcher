## Why

Selection in the launcher's lists is shared between the keyboard and the mouse,
and the mouse keeps winning. Resting the cursor over a row moves the real
selection there, so Enter acts on whatever the pointer happens to cover rather
than on what the user arrowed to. The hover highlight and the selected highlight
are the same `bg-muted` fill, so there is no way to tell which row Enter will
act on. And the selection can be lost entirely: when nothing is selected, Arrow
Down recovers by selecting the first row while Arrow Up does nothing, which
reads as if the caret had swallowed the key.

This is a small, self-contained interaction fix on a surface used on every
single activation, so it is worth doing before more views are built on it.

Milestone: M1 - core. It refines the Svelte rendering shell that M1 delivered;
it adds nothing to the roadmap.

## What Changes

- The pointer never changes the selection by hovering. Only the keyboard and a
  real click move it.
- Hover gets its own quieter treatment, visibly weaker than the selected row, so
  the two can be on screen at once without competing.
- The hover highlight disappears the moment a key is pressed and returns only
  once the pointer really moves, so a resting cursor never paints a row while
  the keyboard drives the list.
- A list with at least one item always has exactly one selected row. The
  selection survives results being replaced: the same item stays selected if it
  is still there, otherwise the first row takes it.
- Arrow keys act only on the list. They never move the caret in the prompt, and
  Arrow Up on the first row keeps that row selected instead of clearing the
  selection.
- The prompt keeps the cursor at all times and stays typeable while the list
  holds its selection.
- The same rules apply to the three list surfaces: root search, a pushed list
  view, and the action panel.
- `src/lib/pointer.svelte.ts` becomes `src/lib/input.svelte.ts`: it answers
  whether hover may paint, and whether a selection change came from the user.
  It no longer decides what is selected.

## Capabilities

### New Capabilities

- `list-navigation`: how a list of rows is selected and navigated - what the
  keyboard owns, what the pointer owns, the invariant that a selection always
  exists, and how selected and hovered rows are told apart.

### Modified Capabilities

<!-- None. Root search and the view protocol keep their requirements; this
     change adds the selection rules they both rely on. -->

## Impact

- `src/App.svelte`: root `Command.Root` and `Command.Item` - pointer selection,
  selection repair when results arrive, hover styling.
- `src/lib/ProtocolView.svelte`: the same for a pushed list view.
- `src/lib/ActionPanel.svelte`: hover no longer moves its selection.
- `src/lib/pointer.svelte.ts` → `src/lib/input.svelte.ts`: pointer-activity and
  user-gesture flags, used for hover painting and for filtering the primitive's
  selection writes.
- No backend, protocol, or manifest change. No new dependency.

Platforms: macOS and Windows. This is frontend-only behaviour with no platform
branch, so one implementation covers both; verification still happens on both.

## Non-goals

- No mouse wheel, drag, or multi-select behaviour.
- No wrap-around at the ends of a list. Arrow Up on the first row and Arrow Down
  on the last row stay put.
- No change to what Enter, Escape, or Ctrl+K do.
- No change to ranking, filtering, or which items a list shows.
- No new design tokens beyond what `src/app.css` already defines.
- No rework of the action panel beyond its hover behaviour.
