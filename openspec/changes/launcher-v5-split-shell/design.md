## Context

See proposal.md for the why. The reference is design system v5,
`Launcher Component System v5.dc.html` (kept outside the repo, in the
author's Downloads). It uses the same herdr colour roles, Geist faces and
Graphite / Paper presets as `ui/theme.slint`, so this change is about shape
and layout, not colour.

Current state:

- `ui/shell/launcher-window.slint` already has `no-frame`, `background:
  transparent`, and one rounded `frame` Rectangle holding SearchBar, view and
  ActionBar. Rounded corners already depend on the transparent window, on
  both renderers.
- `src/slint_shell/render/height.rs` computes the window height from fixed
  part sizes. Only Windows resizes (`keeps_top_edge`); macOS stays at
  `LAUNCHER_MAX_HEIGHT`.
- Windows placement (`position_on_active_display`) reads the width from the
  window rect and the height from `LAUNCHER_MAX_HEIGHT`.
- `State::escape` in `src/ui/state.rs` does pop, clear query, dismiss.
  Root search with an empty query returns the sectioned list today.
- Slint 1.18.1: femtovg draws `drop-shadow-*`; the software renderer's
  `draw_box_shadow` is an empty `TODO`. Windows falls back to software
  without OpenGL 2 (AGENTS.md).

## Goals / Non-Goals

**Goals:**

- Pill and card as two shapes in the one existing window.
- Closed root view as state machine state, so the renderer stays a pure
  renderer of the snapshot.
- No platform code beyond placement and height maths.

**Non-Goals:**

- Detecting which renderer runs. The shadow just disappears under the
  software renderer, which is what the spec allows.
- Click passthrough on Windows (making the gap really click-through).

## Decisions

### One window, two shapes

`LauncherWindow` keeps one transparent window. The single `frame` Rectangle is
replaced by a pill Rectangle (`SearchBar` restyled) and a card Rectangle
holding the view, the answer card, the ActionBar and the action panel. The
card is `visible` only while the snapshot says it is shown.

Rejected:

- **Two native windows** (pill window plus card window). They would need to
  move, show, hide and take focus together on two platforms, against every
  trap in AGENTS.md about focus, `WS_EX_TOOLWINDOW` and NSPanel. Too costly
  for a gap of 8 pixels.
- **`SetWindowRgn` / window shape masks** to cut the gap out of the window.
  Regions have no anti-aliasing, so the rounded ends would be jagged, and it
  is platform code for a visual effect.

### No glass

v5 shows the pill and card with `backdrop-filter: blur`. Slint cannot blur what
is behind an element. DWM backdrops (Acrylic, Mica) and `NSVisualEffectView`
apply to the whole window, not to two rounded shapes in a transparent window.
v5 has glass as an option (`glass ?? true`), so the solid `popover` fill is
within the design.

### Shadows with Slint's drop shadow, plus a window margin

The pill and card use `drop-shadow-blur`, `-offset-y` and `-color` from new
`Theme` tokens. The window grows by a transparent margin on every side
(`Theme.shell-margin`, matching constant in `render/`) so the shadow is not cut.
The visible shell stays 720 wide; the window is `720 + 2 * margin`.

- Windows placement adds `2 * margin` to `LAUNCHER_MAX_HEIGHT` when centring;
  the width already comes from the window rect. Centring stays correct
  because the margin is the same on both sides.
- macOS placement adds the same margin to its frame.
- `height.rs` adds `2 * margin` to every height it returns, and `MAX` keeps
  meaning the visible launcher.

Rejected: **fake shadows from stacked translucent Rectangles**, which would
also work under the software renderer. They cost a lot of elements, look
banded, and the author's machine likely runs femtovg. If it turns out to run
software, revisit then.

### Closed root view lives in the state machine

`State` gets a root-list flag (closed or open). Rules:

- Activation after a reset, a cleared query, and Escape in the open list set
  it closed. Arrow Down on an empty prompt sets it open. A non-empty query
  ignores it.
- Escape: pop a view → clear query → close open list → dismiss.
- Arrow Down in the closed state opens the list. From the bare pill it selects
  the first row. From the "Needs attention" card it then moves one row down,
  through the normal arrow handling.
- The empty-query search still runs as today. When closed, the snapshot shows
  only the pinned section, or no section at all.
- The flag is held by an interrupting hide like the rest of the root state,
  and reset by a deliberate hide.

The snapshot gains one field saying whether the card is shown (`Shell` global
writes it). `height.rs` then counts pill only, or pill + gap + card.

Rejected: **renderer hides the card when the list model is empty**. That
puts an interaction rule in the renderer (AGENTS.md: no interaction rule in
renderer) and could not tell "no results yet" from "closed".

### Clicks on transparent parts

A full-window `TouchArea` sits under the pill and card and sends a new intent
that `Core` handles as an interrupting hide, like focus loss. On macOS, if the
window server passes clicks on fully transparent pixels to the app behind,
that app takes focus and the existing focus-loss hide fires instead. Both
paths meet the spec, so no platform code is needed. Which one macOS does is a
hand check.

### Toast capsule

When the card is hidden, the toast renders as a capsule under the pill, using
the same `Shell` toast properties the ActionBar reads. `height.rs` adds the
capsule's height and a gap while it shows, so on Windows the window is tall
enough. No state machine change: placement depends only on the card flag
already in the snapshot.

### Components

| Surface | Part | Note |
|---|---|---|
| Pill | `ui/shell/search-bar.slint` | restyled; back control and filter pill inside |
| Card | `ui/shell/launcher-window.slint` | new Rectangle, no new component |
| Primary action chip | `ui/shell/action-bar.slint` + `ui/atoms/kbd.slint` | restyled |
| Inset selected row | `ui/views/list.slint`, `ui/shell/action-panel.slint` | rounded fill with side inset |
| Toast capsule | `ui/shell/launcher-window.slint` | reuses `Dot`, spinner and `Kbd` atoms from the ActionBar's toast, inside a rounded Rectangle |

No new component file: the capsule is a few elements in the window, built from
existing atoms. If it grows, it moves to `ui/shell/toast.slint`.

### Text

No new or changed user-visible strings. "Needs attention", "Suggestions",
"Everything else", and all toast texts stay as they are.

### Contracts

No change to the extension manifest or the view protocol.

## Risks / Trade-offs

- [The author's Windows machine runs the software renderer, so no shadows] →
  The spec allows it. Check the renderer during the first hand check. If it
  is software, decide then whether fake shadows are worth it.
- [macOS full-height transparent window catches clicks below the pill] → The
  transparent-click hide covers it either way. Hand check.
- [Bare pill hides recent items, and discovery suffers] → Arrow Down opens
  them. Revisit after some daily use.
- [`ITEM_LIMIT` and fixed heights] → Row heights stay 44, so counting in
  `render/` still works. Only pill, gap and margin constants are new. Change
  them in `Theme` and `render/` together.
- [Height maths and placement drift between platforms] → `height.rs` is
  shared and unit-tested. Placement gets one task per platform.

## Migration Plan

None. Alpha, no stored data or config touched. Rollback is reverting the
commits.
