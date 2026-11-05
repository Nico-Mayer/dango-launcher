# Proposal

## Why

The pill and the card float over other apps with no shadow, so they look flat
and edges blur into light backgrounds. The v5 split shell left shadows for a
follow-up, which needed a GPU renderer on Windows. The author's Windows
machine now runs femtovg (AMD OpenGL driver loads; the action panel's existing
shadow draws), so the follow-up can land.

## What Changes

- The pill and the card each cast the action panel's shadow: 24 blur, 8 down,
  in the theme's shadow colour.
- The launcher window gets a transparent margin on every side, wide enough for
  the shadow. The pill and the card move inside it; their sizes, the gap, and
  their place on screen stay the same.
- On Windows the window's height still follows the visible launcher, plus the
  margin above and below.
- A click on the margin counts as a click outside, like the gap.
- Where Slint falls back to its software renderer (Windows without OpenGL 2),
  no shadow draws. Accepted gap, stated in the spec.

## Non-goals

- No glass, blur, or vibrancy behind pill or card.
- No native window shadow (DWM, `NSWindow.hasShadow`): it would trace the
  whole rectangular window, not two rounded shapes.
- No Skia renderer to get shadows under the software fallback.
- No fake shadows from stacked rectangles.
- No click-through on the margin or the gap. Clicks there still reach the
  launcher and dismiss it.
- No change to the action panel or dropdown shadows, or to the notice window.
- No shrinking macOS window (`keeps_top_edge` stays false there).

## Platforms

Windows and macOS. Both draw with femtovg. The Slint window adds the margin to
its own size, so macOS gets it with no platform code, and its placement reads
the window's frame. Windows' native resize and placement add the margin. Both
get a hand check.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `design-system`: the pill and the card cast a shadow; gap on the software
  renderer.
- `launcher-shell`: the window has a transparent margin around the launcher;
  window height rules and a click on the margin.

## Impact

- `ui/theme.slint`: shadow size and margin values.
- `ui/shell/launcher-window.slint`, `ui/shell/action-panel.slint`: shadows,
  the margin, layout inside it.
- `src/slint_shell/mod.rs`: the Windows native resize adds the margin.
- `src/platform/mod.rs`, `src/platform/windows/mod.rs`: placement height
  includes the margin.
- `src/slint_shell/tests.rs`, `src/slint_shell/render/mod.rs`
  (`render_headless`): window size and absolute positions move by the margin.
- `.design/Launcher Component System v5.dc.html`: shows the shadows again.
