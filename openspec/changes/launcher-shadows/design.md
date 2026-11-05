# Design

## Context

- `LauncherWindow` (`ui/shell/launcher-window.slint`) is `Theme.window-width`
  (720) wide and `Shell.height` tall. Pill at y 0, card below the gap, both
  edge to edge. The card's height is `min(Shell.launcher-height, root.height)
  - self.y`: the `root.height` term keeps the card inside the window when a
  resize lands a frame before the new height. A full-window `TouchArea` turns
  clicks on transparent parts into `Intents.clicked-outside`.
- `render/height.rs` computes the visible launcher height (max
  `LAUNCHER_MAX_HEIGHT`, 480). `set_launcher_height` (`src/slint_shell/mod.rs`)
  returns early when `Shell.height` already equals it, and on macOS
  (`!keeps_top_edge`) before writing anything, so there `Shell.height` stays
  at its default 480 (`ui/globals.slint`). On Windows it writes `Shell.height`
  and resizes the native window with `SWP_NOMOVE`.
- Placement: `launcher_origin` centres a window of a given size. Windows passes
  the real width and `LAUNCHER_MAX_HEIGHT` scaled to the target DPI; macOS
  passes the window frame's size.
- `ActionPanel` already has `drop-shadow-blur: 24px`, `drop-shadow-offset-y:
  8px`, `Theme.shadow`. The SearchBar filter dropdown has 16 / 4.
- Renderer: femtovg draws `drop-shadow-*`; Slint 1.18.1's software renderer
  draws none. Confirmed femtovg on the author's Windows machine (debug build
  loads `atio6axx.dll`; pixels below the action panel fade 217 to 229 on a 255
  card).
- Slint lowers `drop-shadow-*` into a sibling `BoxShadow` element before the
  `clip` pass (`i-slint-compiler-1.18.1/passes/lower_shadows.rs`), so the
  card's `clip: true` does not cut its own shadow, and a hidden card hides its
  shadow too.

## Goals / Non-Goals

**Goals:**

- Pill and card cast the panel's shadow, uncut by the window edge.
- Pill and card stay at the same place on screen and the same size.
- No new platform code beyond the height constant both platforms already use.

**Non-Goals:** see proposal.

## Decisions

### One shadow, shared through `Theme`

Add `Theme.shadow-blur` (24px) and `Theme.shadow-offset` (8px). `ActionPanel`,
pill, and card use them. The dropdown keeps its own smaller 16 / 4; it is a
menu, not a surface. Author chose the panel's values over larger or smaller
ones.

### A uniform margin of 32 logical pixels

The shadow reaches `blur` past each edge, shifted `offset` down: 16 above, 24
at the sides, 32 below. One margin on all four sides, `Theme.shadow-margin` =
blur + offset = 32, keeps the window symmetric around the launcher. So
`launcher_origin` still centres the visible launcher with no change to its
formula: it only needs the window's full size. Placement and the Windows top
edge keep working as they are.

Rejected: **per-side margins (16 / 24 / 24 / 32).** Saves a few transparent
pixels but makes centring offset-aware on both platforms. Not worth it.

### `Shell.height` stays the launcher's; the Slint window adds the margin

`render/height.rs` and `Shell.height` keep meaning the visible launcher height.
`LauncherWindow` sets `width: Theme.window-width + 2 * Theme.shadow-margin` and
`height: Shell.height + 2 * Theme.shadow-margin`. So:

- macOS gets a 544 high window from the existing 480 default, with no Rust
  change, and its placement reads that frame.
- `set_launcher_height`'s early return (`get_height() == height`) keeps
  matching, so Windows does not resize on every snapshot.
- On Windows, `set_launcher_height` only passes
  `height + 2 * LAUNCHER_SHADOW_MARGIN` to `platform.set_height`.

`LAUNCHER_SHADOW_MARGIN` (32.0) sits next to `LAUNCHER_MAX_HEIGHT` in
`src/platform/mod.rs`, matching `Theme.shadow-margin` (AGENTS.md: change
`Theme` and the matching constant together). Windows' placement height becomes
`LAUNCHER_MAX_HEIGHT + 2 * LAUNCHER_SHADOW_MARGIN`.

Rejected: **writing `height + 2 * margin` to `Shell.height`.** macOS never
writes `Shell.height`, so its window would stay 480 and cut the card's bottom,
and the early return would never match.

### One container inside the margin

Pill and card move into one plain `Rectangle` at `x`/`y`:
`Theme.shadow-margin`, `width: Theme.window-width`, `height: parent.height -
2 * Theme.shadow-margin`, with no `TouchArea`. Pill and card keep their
coordinates. The card's clamp keeps its window term, reading `parent.height`
instead of `root.height`: `min(Shell.launcher-height, parent.height) -
self.y`. The full-window `TouchArea` stays outside the container and covers
the margin, so a click there is `clicked-outside` with no new code.

Rejected: **moving pill and card by the margin one by one.** More lines change,
and the clamp has to subtract the margin itself.

Design-system parts: pill and card in `ui/shell/launcher-window.slint`,
`ActionPanel` in `ui/shell/action-panel.slint`. No new component.

### Rejected alternatives

- **Native window shadow** (`DWMWA_*`, `NSWindow.hasShadow`): traces the whole
  rectangular window, not the two rounded shapes, and not the gap.
- **`renderer-skia`**: would draw shadows where OpenGL 2 is missing, but adds
  Skia binaries to the build for machines the author does not use. Revisit if
  the software fallback shows up on a daily machine.
- **Stacked translucent rectangles**: rejected in launcher-v5-split-shell;
  many elements and visible bands.

No manifest or view protocol change. No user-visible text.

## Risks / Trade-offs

- No automated test covers the margin maths in `set_launcher_height` or
  Windows placement: the test harness bypasses `set_launcher_height`, and the
  `launcher_origin` tests do not involve the margin. The hand checks cover
  them.
- The margin blocks clicks to the app behind within 32 pixels of the launcher.
  Same as the gap today; they dismiss the launcher, as the spec says.
- The pill's shadow falls into the 8px gap and darkens it slightly. The app
  behind still shows through; spec scenario updated.
- The software renderer draws no shadow. Accepted, stated in the spec.
- Placement clamps the window's top at the work area's top, so on a very short
  display the visible launcher can sit up to 32 pixels lower than before. Only
  the existing clamp test covers it; fine for a hobby project.
