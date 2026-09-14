## Why

This is **M4 - windows** from `openspec/ROADMAP.md`: the `window-management`
extension. A keyboard-first launcher that can lock the screen and paste a snippet
but cannot move the window in front of it is missing the feature its users reach
for most. Tiling the focused window to a half, a quarter, or a third, maximising
it, centring it, and throwing it to the next display are the daily moves a
Raycast or Wox user expects.

The plumbing this needs already landed. M3 records the window that was focused
before the launcher appeared, which is exactly the window these commands act on,
and the project already carries the Windows geometry traps this feature lives or
dies by: correcting frames with `DWMWA_EXTENDED_FRAME_BOUNDS` and being
per-monitor DPI aware. Building it now, on the author's daily-use machine, turns
that groundwork into the first feature that reshapes other applications.

## What Changes

- A `window-management` built-in extension contributing static commands, one per
  move: left/right/top/bottom halves, the four quarters, left/centre/right
  thirds, maximise, centre, and move to the next display.
- A new `WindowManager` platform trait for reshaping a window other than the
  launcher's own: read the focused window's frame and its monitor's work area,
  and set the window's frame. Platform code sits in `platform/windows` and
  `platform/macos` behind it, as every other OS integration does.
- The target of every command is the window that was focused before the launcher
  appeared, reusing the previous-foreground the launcher already records rather
  than acting on the launcher itself.
- Shared, platform-neutral geometry: the halves, quarters, thirds, maximise,
  centre, and next-display rectangles are computed once from a work area, so
  only reading and writing a window's frame is platform-specific.
- On Windows, frames are corrected with `DWMWA_EXTENDED_FRAME_BOUNDS` so tiled
  windows leave no gaps, and all geometry is in physical pixels with per-monitor
  DPI awareness.
- On macOS, the same commands set the focused window's position and size through
  the Accessibility API, gated behind the Accessibility permission M3 surfaces.

## Capabilities

### New Capabilities

- `window-management`: what each move does to the focused window, which window is
  the target, how a window is placed exactly against a display's work area on
  each platform, what happens across multiple displays and DPI scales, and how
  the feature behaves when there is no window to move or the permission is
  missing.

### Modified Capabilities

None.

## Impact

- **Affected code**: `src-tauri/src/platform/` (the new `WindowManager` trait and
  a `windows` and `macos` implementation), `src-tauri/src/extensions/window_management/`
  (new: manifest, commands, the shared geometry), and `src-tauri/src/lib.rs`
  (registering the extension and wiring the target-window source).
- **Platforms**: Windows and macOS. Windows is built and verified first, on the
  author's daily-use machine. macOS ships behind the Accessibility permission and
  is verified after, the same split M3 used.
- **Dependencies**: none new. Windows uses the `windows`/`windows-sys` surface
  already vendored; macOS uses the `objc2-application-services` Accessibility API
  M3 already added.
- **Permissions**: macOS reuses the Accessibility state from M3; no new
  permission. Windows needs none, except the elevated-window ceiling already
  documented, which applies here too since a non-elevated process cannot reshape
  an elevated window.
- **View protocol**: unchanged. Every command is a direct, no-view action, so no
  new `FieldKind`, view, or `protocolVersion` bump.

## Non-goals

- **No per-command hotkeys.** The roadmap lists them under M4, but binding a
  command to a global shortcut needs the recorder UI and conflict detection M5
  owns, and M3 already set the precedent of deferring hotkeys to M5. Here the
  commands are found by typing in root search. Nothing in this design resists a
  hotkey being added later; the commands are ordinary registry entries.
- **No window layouts or grids beyond the fixed set.** No arbitrary N-by-M grid,
  no custom fractions, no saved layouts. The listed moves cover the daily need;
  more is scope this milestone does not carry.
- **No focus-follows or window-switching.** This moves the window the user is in;
  it does not raise, cycle, or hunt other windows.
- **No multi-window or whole-application arrangement.** One command reshapes one
  window, the focused one.
- **No snapping to other windows or edge-drag gestures.** Placement is against a
  display's work area, not against neighbouring windows.
- **No Linux.** Out of scope for the project.
