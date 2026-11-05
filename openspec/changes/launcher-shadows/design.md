# Design

## Context

- `LauncherWindow` (`ui/shell/launcher-window.slint`) is `Theme.window-width`
  (720) wide and `Shell.height` tall. Pill at y 0, card below the gap, both
  edge to edge. A full-window `TouchArea` turns clicks on transparent parts
  into `Intents.clicked-outside`.
- `render/height.rs` computes the visible launcher height (max
  `LAUNCHER_MAX_HEIGHT`, 480). `set_launcher_height` (`src/slint_shell/mod.rs`)
  writes it to `Shell.height` and, on Windows, resizes the native window with
  `SWP_NOMOVE`. macOS keeps the component at 480.
- Placement: `launcher_origin` centres a window of a given size. Windows passes
  the real width and `LAUNCHER_MAX_HEIGHT` scaled to the target DPI; macOS
  passes the window frame's size.
- `ActionPanel` already has `drop-shadow-blur: 24px`, `drop-shadow-offset-y:
  8px`, `Theme.shadow`. The SearchBar filter dropdown has 16 / 4.
- Renderer: femtovg draws `drop-shadow-*`; Slint 1.18.1's software renderer
  draws none. Confirmed femtovg on the author's Windows machine (debug build
  loads `atio6axx.dll`; pixels below the action panel fade 217 to 229 on a 255
  card).

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

### Height stays the launcher's; the window adds the margin

`render/height.rs` keeps returning the visible launcher height, so UI tests and
the height rules keep their meaning. `set_launcher_height` writes
`height + 2 * margin` to `Shell.height` and to `platform.set_height`. A
`LAUNCHER_SHADOW_MARGIN` constant sits next to `LAUNCHER_MAX_HEIGHT` in
`src/platform/mod.rs`, matching `Theme.shadow-margin` (AGENTS.md: change
`Theme` and the matching constant together). Windows' placement height
becomes `LAUNCHER_MAX_HEIGHT + 2 * LAUNCHER_SHADOW_MARGIN`. macOS reads the
frame, so it follows on its own.

### Layout inside the margin

`LauncherWindow` width becomes `window-width + 2 * shadow-margin`. The pill and
the card get `x: Theme.shadow-margin`, `width: Theme.window-width`, and their
`y` moves down by the margin. The card's height clamp reads the launcher
height, not `root.height`, minus the margin. The full-window `TouchArea`
already covers the margin, so a click there is `clicked-outside` with no new
code.

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

- The card has `clip: true`. If Slint clips its own drop shadow with it, wrap
  the card in a parent `Rectangle` that carries the shadow. The first UI task
  checks this with a screenshot.
- The margin blocks clicks to the app behind within 32 pixels of the launcher.
  Same as the gap today; they dismiss the launcher, as the spec says.
- The pill's shadow falls into the 8px gap and darkens it slightly. The app
  behind still shows through; spec scenario updated.
- The software renderer draws no shadow. Accepted, stated in the spec.
- Placement clamps the window's top at the work area's top, so on a very short
  display the visible launcher can sit up to 32 pixels lower than before. Only
  the existing clamp test covers it; fine for a hobby project.
