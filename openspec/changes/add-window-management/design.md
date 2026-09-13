## Context

See proposal.md for why. What already exists and shapes this:

- `platform::LauncherWindow` manages the launcher's own window and nothing else.
  Window management reshapes a different window, so it needs its own trait rather
  than an extension of that one.
- `platform::active_monitor` and `launcher_origin` already resolve the display a
  window is on and compute a placement, and `tauri::Monitor` gives `work_area`
  and `scale_factor`. The launcher's Windows positioning already projects between
  logical and physical pixels through a monitor's scale factor; the same problem
  recurs here.
- M3's Windows code records the window that was foreground before the launcher
  appeared (`platform::windows::remember_previous_foreground`, read back inside
  the platform module) and already checks the elevated-window ceiling by
  comparing integrity levels. Both are exactly what this feature's target
  selection and failure reporting need.
- The `system` extension is the model for a command-only built-in: a `Manifest`
  with `CommandDecl`s in `InvocationMode::NoView`, a `Command` per id, and
  `perform_action` for anything that pushes a view. Window management pushes no
  view, so it is `system` without the confirmation dialogs.
- The project context records the two Windows window traps this feature is most
  exposed to: correct geometry with `DWMWA_EXTENDED_FRAME_BOUNDS` or invisible
  resize borders leave gaps, and per-monitor DPI awareness is required for
  multi-monitor tiling.

## Goals / Non-Goals

**Goals:**

- One narrow platform trait for reshaping a window, with all the geometry maths
  shared and platform-neutral, so the part that is hard to test on the machine
  that cannot compile it is as small as possible.
- Pixel-exact tiling: a window tiled to a half meets the work area with no gap
  and does not cover the taskbar, on both platforms and across DPI scales.
- The commands act on the window the user was actually in, never the launcher.

**Non-Goals:**

- See proposal.md. At design level additionally: no persistence of window state,
  no undo, and no attempt to detect or preserve a window's snapped or maximised
  OS state. Each command computes an absolute target rectangle and applies it.

## Decisions

### A new `WindowManager` trait, separate from `LauncherWindow`

`LauncherWindow` owns the one window Dango created. Window management reshapes an
arbitrary foreign window, which is a different capability with a different target
and different permission story. Folding it into `LauncherWindow` would widen a
trait that has one clear job.

The trait is deliberately narrow, three questions and one action:

- the target window's current frame,
- the work area of the display that window is on,
- the set of displays in order (for move-to-next-display),
- set the target window's frame to a rectangle.

Everything else, which region a command maps to, is computed above the trait
from a work area and the current frame. So the platform code is only "read a
frame, read a monitor, write a frame", and the halves, quarters, thirds,
maximise, centre, and next-display maths live once in a shared, unit-tested
module. This is the same split M3 used for the clipboard round trip: one tricky
platform seam, the logic on top of it shared and tested.

Rejected: computing rectangles inside each platform implementation. It would
duplicate the arithmetic on the platform that cannot be compiled locally and
double the surface where a thirds-rounding bug could hide.

### The target is the previously focused window, not the foreground at apply time

By the time a command runs the launcher is the foreground window, so reading the
foreground then would reshape the launcher. The target is the window that was
focused when the launcher was summoned, which M3 already records on Windows. The
command captures that target before the launcher hides and acts on it after.

Rejected: reshaping whatever is foreground after the launcher hides. The hide is
asynchronous and racy, exactly the trap M3 documented, and it would sometimes
catch the wrong window. Using the remembered previous window is deterministic.

### Geometry is computed in the display's own pixel space

Each command turns a work area and, for centre, the window's current size, into a
target rectangle. Halves and quarters split the work area in two or four; thirds
split the width into three and take full height; maximise is the work area;
centre keeps the size and centres the origin; move-to-next-display finds the
window's current display, picks the next in order, and maps the same relative
region onto the destination work area.

Coordinates follow each platform's convention rather than a shared one, because
the launcher already learned they differ: Windows works in physical pixels across
one virtual desktop, macOS in logical points per screen with a bottom-left
origin. The trait's frame getter and setter speak the platform's own coordinates,
and the shared maths operates on whatever work area the trait reports, so no
conversion crosses the seam. This mirrors the launcher's rule that positioning
code is not shared between the platforms.

### Windows corrects for the invisible frame with `DWMWA_EXTENDED_FRAME_BOUNDS`

`GetWindowRect` on Windows returns a rectangle larger than what the user sees,
because it includes the invisible resize border. Placing a window by that
rectangle leaves a visible gap on tiled windows. So the Windows implementation
reads `DWMWA_EXTENDED_FRAME_BOUNDS` to learn the visible frame, computes the
difference from the `GetWindowRect` frame, and offsets the `SetWindowPos` call by
it so the visible edges land where intended. This is the trap the project context
names, applied at the one place it matters.

Placement is per-monitor DPI aware: the work area and the target rectangle are in
physical pixels, and moving to a display of a different scale uses that display's
own metrics, so a window keeps its intended region rather than the size the old
display's scale implied.

Rejected: `SetWindowPlacement` with a normal-position rectangle. It is in
workspace coordinates and interacts with the min/max/restore state in ways that
fight an absolute tile; `SetWindowPos` after clearing any maximised state is the
predictable primitive. A maximised window is restored first so a subsequent tile
takes effect rather than being ignored.

### macOS sets the frame through the Accessibility API

macOS reshapes the focused window through the accessibility element M3 already
reaches: the focused application's `AXWindow`, setting `kAXPositionAttribute` and
`kAXSizeAttribute`. It is gated behind the Accessibility permission, and the
missing-permission state is the one M3 built; this feature reuses it rather than
inventing a second. Reading the display set uses `NSScreen`, whose frames are the
logical points the AX setter expects.

Rejected: a private or undocumented window-move path. The AX route is the
supported one, it is already permissioned, and it is what the launcher's own
selection reading proved works.

### The commands are ordinary registry entries, hotkeys deferred

Each move is a `CommandDecl` in `InvocationMode::NoView`: invoked from root
search, it acts and the launcher gets out of the way. No view is pushed, so the
view protocol does not move and `manifestVersion` stays 1. Per-command hotkeys,
though listed under M4 in the roadmap, are left to M5 with its recorder and
conflict detection, the same call M3 made for snippets and quicklinks. Nothing
here resists a hotkey later: a bound key would resolve to the same command id.

This locks nothing in the versioned contracts: no new manifest field, no new view
kind, no new `FieldKind`. It is additive at the extension level only.

### No UI surface

Every command is a direct action with no view, so there is no Bits UI primitive
to name and no new frontend surface. The commands appear as ordinary results in
the root search `Command` list that already exists.

## Risks / Trade-offs

- **Windows lands unverified until it runs.** As with M3, nothing here compiles
  on macOS and the geometry traps only show at runtime. → Symbol names and module
  paths are checked against the vendored crate source before pushing, CI compiles
  it, and a driven check on the Windows machine closes the confirmation tasks,
  the arrangement M3 used.
- **`DWMWA_EXTENDED_FRAME_BOUNDS` is the whole correctness of tiling on Windows.**
  Get it wrong and every tiled window is off by the border width. → It is
  verified by measuring that tiled edges are flush, not by eye alone.
- **Some windows refuse to resize to arbitrary rectangles.** A window with a
  minimum size, or a fullscreen-exclusive one, will not honour the frame. → Out
  of reach and stated: the command applies the frame and the window keeps what it
  can. This is the ceiling every tiling tool shares.
- **Move-to-next-display across mixed DPI is the subtle case.** → The maths works
  from the destination display's own work area and scale, and it is the scenario
  the spec calls out for verification.
- **macOS depends on a permission that may be revoked.** → It reuses M3's
  detection and prompt, so a revoked permission produces the same explained
  failure rather than a silent no-op.
