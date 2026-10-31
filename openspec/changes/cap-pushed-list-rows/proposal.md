## Why

A command's list view has no row limit, but the renderer builds every row and
replaces the whole model on every keystroke. Clipboard History pushes up to 500
entries, many with full-size image thumbnails, so opening and filtering it costs
work in proportion to the whole history (audit finding A9-05). Root search
already stops at 50 results. The cap is also what makes the icon cache fix
(A2-01) safe: once file icons are no longer kept forever, each snapshot may only
decode the thumbnails it actually shows.

## What Changes

- After the launcher has filtered a pushed list or grid against the query, the
  view shows at most 50 items, counted in the order they are drawn across all
  sections. Root search keeps its existing limit of 50.
- Typing narrows the items further, so an item past the first 50 is reached by
  filtering, as in root search.
- The cap applies whoever filters: a view whose command owns filtering is also
  shown with at most 50 items.
- No change to the view tree format or the protocol version: a command may
  still send any number of items. Only what the launcher shows is bounded.

## Non-goals

- No "show more" row, item count, or paging. Typing to narrow is the only way
  past the cap, as in root search.
- No lazy or virtualised list. The renderer keeps building every row it gets
  (see the ListView note in AGENTS.md).
- No change to how many entries Clipboard History stores or sends.
- No change to forms or detail views.
- The icon cache fix (A2-01) is not part of this change. It follows once this
  lands.

## Platforms

Windows and macOS. The cap lives in the platform-independent launcher state
machine (`src/ui`), so both platforms get it from the same code.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `view-protocol`: a list or grid view shows at most 50 items after the
  launcher's filtering.

## Impact

- `src/ui/state.rs`: the grouping that feeds a pushed list or grid stops after
  50 items in draw order. Selection, the action panel, and the window height
  only ever see those items.
- `src/ui/tests.rs`: tests for the cap with sections, with filtering, and for a
  command-filtered view.
- No change to `src/protocol.rs`, the renderer, or any extension.
- Unblocks audit finding A2-01 in `src/slint_shell/icons.rs`.
