## Context

See proposal.md for the why. The reference is design system v5,
`Launcher Component System v5.dc.html`, kept outside the repo in the author's
Downloads. It uses the same herdr colour roles, Geist faces and Graphite /
Paper presets as `ui/theme.slint`, so this change is about shape and layout,
not colour.

Current state:

- `ui/shell/launcher-window.slint` already has `no-frame`, `background:
  transparent`, and one rounded `frame` Rectangle holding SearchBar, view and
  ActionBar. Rounded corners already depend on the transparent window, on
  both renderers.
- `ui/views/list.slint` already draws selected and hovered rows as rounded
  fills inset 6px from the edge, as v5 shows. Nothing to do there.
- `src/slint_shell/render/height.rs` computes the launcher height from fixed
  part sizes. `set_launcher_height` (`src/slint_shell/mod.rs`) returns early
  when `!keeps_top_edge`, so on macOS `Shell.height` stays at
  `LAUNCHER_MAX_HEIGHT` and the window keeps its size.
- Placement on both platforms reads the size from the window itself (Windows
  takes the height from `LAUNCHER_MAX_HEIGHT`). Without a shadow margin,
  neither changes.
- `State::escape` (`src/ui/state.rs`) does pop, clear query, dismiss.
  `root_selected()` falls back to the first item of `root_order()`, which
  walks every result. Root search with an empty query returns the sectioned
  list today.
- `State` survives `Event::Hidden` untouched, and `reset()` clears it. Holding
  and resetting new root state comes for free.
- Slint 1.18.1: femtovg draws `drop-shadow-*`; the software renderer's
  `draw_box_shadow` is an empty `TODO`. Windows falls back to software
  without OpenGL 2 (AGENTS.md).

## Goals / Non-Goals

**Goals:**

- Pill and card as two shapes in the one existing window.
- Closed root view as state machine state, so selection, Enter, the action
  panel, bindings and the snapshot can never disagree.
- No platform code.

**Non-Goals:**

- Shadows and the window margin they need (follow-up change).
- Click passthrough on Windows (making the gap truly click-through).

## Decisions

### One window, two shapes

`LauncherWindow` keeps one transparent window. The single `frame` Rectangle is
replaced by a pill Rectangle (`SearchBar` restyled) and a card Rectangle
holding the view, the answer card, the ActionBar and the action panel. The
card is `visible` only while the snapshot says it is shown, and its height
comes from `Shell`, not from the window.

Rejected:

- **Two native windows** (pill window plus card window). They would need to
  move, show, hide and take focus together on two platforms, against every
  trap in AGENTS.md about focus, `WS_EX_TOOLWINDOW` and NSPanel. Too costly
  for a gap of 8 pixels.
- **`SetWindowRgn` / window shape masks** to cut the gap out of the window.
  Regions have no anti-aliasing, so the rounded ends would be jagged, and it
  is platform code for a visual effect.

### Card height on both platforms, window height on Windows only

The renderer writes a card height computed by `height.rs` to `Shell` on both
platforms. `set_launcher_height` keeps resizing the window only where
`keeps_top_edge` holds. On macOS the window stays 480 tall and the area below
the card is transparent, so a short list looks the same on both platforms.
This is the same `height.rs` work as Windows needs anyway.

Rejected: **card fills the window on macOS**, as `frame` does today. With the
split shell, a three-row list would show a tall empty card with the ActionBar
far below the rows.

### No glass, no shadows (for now)

v5 shows the pill and card with `backdrop-filter: blur`. Slint cannot blur what
is behind an element. DWM backdrops (Acrylic, Mica) and `NSVisualEffectView`
apply to the whole window, not to two rounded shapes in a transparent window.
v5 has glass as an option (`glass ?? true`), so the solid `popover` fill is
within the design.

Shadows need a transparent window margin, so window size, placement on Windows
and `height.rs` all change, and the software renderer draws none of it. The
hand check in this change records which renderer the author's machine runs;
a follow-up adds shadows if it is femtovg.

Rejected: **fake shadows from stacked translucent Rectangles**, which would
also work under the software renderer. They cost a lot of elements and look
banded.

### Closed root view lives in `root_order()`

`State` gets a root-list flag, closed or open. `root_order()` honours it: with
an empty query and the list closed it yields only pinned items, or nothing.
Everything that reads root rows goes through it: `root_selected()`, `confirm`,
the action panel's actions, `bindings`, the ActionBar's context, the snapshot.
So the bare pill has no selection, Enter and Ctrl+K do nothing, and no
hidden item's shortcut fires.

Rules:

- `reset()`, a query that becomes empty, and Escape in the open list set it
  closed. Arrow Down on an empty prompt sets it open. A non-empty query
  ignores it.
- Escape: pop a view, else clear query, else close the open list, else
  dismiss.
- Arrow Down while closed sets it open. From the bare pill the selection then
  falls back to the first row. From the "Needs attention" card it then runs
  the normal one-row step. Arrow Up while closed steps over pinned rows or
  over nothing, so it needs no rule.
- The empty-query search runs as today; only `root_order()` filters.

The snapshot gains one field saying whether the card is shown, written to the
`Shell` global. `height.rs` counts pill only, or pill + gap + card.

Rejected:

- **Filter in the snapshot only.** The state machine would still select a
  hidden first suggestion: Enter would launch it and Ctrl+K open its panel.
- **Renderer hides the card when the list model is empty.** That puts an
  interaction rule in the renderer and cannot tell "no results yet" from
  "closed".

### Click in the gap

A `TouchArea` behind the pill and card, covering the window, sends a new
intent that `Core` handles as an interrupting hide that gives focus back,
as the hotkey does. Dango still holds the foreground after the click, so
leaving the choice to Windows could activate an always-on-top window instead
of the app the user came from. On macOS the window server likely passes
clicks on fully transparent pixels to the app behind; that app then takes
focus and the focus-loss hide fires. Either way the launcher hides and holds,
so no platform code. The macOS behaviour is a hand check.

Rejected: **reuse `hide_on_focus_loss`**. It skips the focus restore, which
is right only when another app already has the foreground.

### Toast at the pill's end

With the card hidden, the toast draws at the right end of the pill, where a
pushed view's filter pill sits. Root has no filter, so the two never meet. It
uses the same `Shell` toast properties and atoms (`Dot`, spinner, `Kbd`) as
the ActionBar's toast. No height change, so the window never grows or shrinks
for a toast.

Rejected: **capsule under the pill**, as v5 draws it. It needs its own height
in `height.rs` and makes the window on Windows grow and shrink as toasts come
and go.

### Test helpers

`Harness::root` (`src/slint_shell/tests.rs`) and `root(count)`
(`src/ui/tests.rs`) seed root with an empty query, and about a hundred tests
then expect rows. The helpers press Arrow Down once after seeding, in the same
commit that closes the root view, so each commit still passes and single
tests stay untouched. Tests of the closed view itself seed without it.

### Components

| Surface | Part | Note |
|---|---|---|
| Pill | `ui/shell/search-bar.slint` | restyled; back control, filter pill and pill-end toast inside |
| Card | `ui/shell/launcher-window.slint` | new Rectangle, no new component |
| Primary action chip | `ui/shell/action-bar.slint` + `ui/atoms/kbd.slint` | restyled |

The pill-end toast shares its look with the ActionBar's toast. If the two
copies differ by more than position, the shared part moves to
`ui/shell/toast.slint`.

### Text

No new or changed user-visible strings. "Needs attention", "Suggestions",
"Everything else", and all toast texts stay as they are.

### Contracts

No change to the extension manifest or the view protocol.

## Risks / Trade-offs

- [Bare pill hides recent items, and discovery suffers] → Arrow Down opens
  them. Revisit after some daily use.
- [A long toast at the pill's end crowds the query] → It is cut off with an
  ellipsis before the query (design-system spec). Long toasts are rare on an
  empty pill.
- [macOS passes or does not pass clicks through transparent pixels] → Both
  paths hide the launcher. Hand check.
- [Fixed heights] → Row heights stay 44, so counting in `render/` still works.
  Only pill and gap constants are new. Change them in `Theme` and `render/`
  together.

## Migration Plan

None. Alpha, no stored data or config touched. Rollback is reverting the
commits.
