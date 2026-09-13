## Why

A follow-on to M4's `window-management` extension, which now ships tiling,
maximise, centre, and move-to-next-display on both platforms. Daily use wants
two more things it does not yet have: sizes beyond the fixed regions (a roomy
almost-maximised window, a comfortable default, nudging bigger and smaller,
a centred column), and the Raycast habit where pressing the same tiling command
again cycles its width or height through a few useful fractions instead of doing
nothing.

These are all arithmetic on the display's work area and the window's current
frame, which the extension already reads. So this rides entirely on the plumbing
M4 built: no new platform code, no trait change, and it lands on Windows and
macOS at once because both already read and write a window's frame.

## What Changes

- New size commands, each a no-view command like the existing ones: almost
  maximise, reasonable size, make larger, make smaller, and centre half.
- Repeating a tiling command cycles its size. Left and right half cycle their
  width through 1/2, 2/3, 1/3; top and bottom half cycle their height the same
  way; centre cycles both dimensions. The first press still does what it does
  today, so nothing existing regresses.
- Make larger and make smaller grow or shrink the focused window by a step
  around its own centre, clamped to the work area and to a sane minimum.
- A small, platform-neutral cycle state in the extension remembers the last
  command and the frame it produced, so a repeat advances the cycle while any
  other change to the window resets it.

## Capabilities

### Modified Capabilities

- `window-management`: gains the new size commands and the repeat-to-cycle
  behaviour of the tiling commands. The existing requirements are unchanged; this
  adds requirements alongside them.

## Impact

- **Affected code**: `src-tauri/src/extensions/window_management/geometry.rs` (the
  new size rectangles and the step maths, with tests) and
  `src-tauri/src/extensions/window_management/mod.rs` (the new commands and the
  cycle state). No change to the `WindowManager` trait or to either platform's
  implementation.
- **Platforms**: Windows and macOS both, with no platform-specific work, since
  the feature is geometry over the frame and work area the trait already
  provides. Verified on Windows by extending the existing driven walkthrough;
  macOS inherits it and is confirmed by hand.
- **Dependencies**: none new.
- **View protocol**: unchanged. Every command stays a no-view action.

## Non-goals

- **No virtual desktop management.** Switching or moving windows across virtual
  desktops was considered and dropped: Windows exposes only an undocumented,
  per-build COM interface for switching, and macOS has no supported Spaces API at
  all, so it cannot ship cleanly on both.
- **No per-command hotkeys.** Still M5, with its recorder and conflict detection.
- **No configurable fractions or step size.** The cycle is fixed at 1/2, 2/3,
  1/3 and the step is a fixed fraction of the work area. Making these preferences
  waits for M7's preferences window.
- **No new regions beyond those listed.** No arbitrary grid, no custom fractions,
  no saved layouts.
- **No Linux.** Out of scope for the project.
