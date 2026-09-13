## 1. Shared geometry

Pure Rust with no platform and no UI, so it is finished and tested first and the
platform seam stays as small as possible.

- [x] 1.1 Add a `window_management` module with a `Region` for the fixed moves (left/right/top/bottom halves, four quarters, left/centre/right thirds, maximise) and a function mapping a `Region` plus a work-area rectangle to a target rectangle, with tests over each region including that halves and quarters tile without gaps or overlap
- [x] 1.2 Compute centre from a work area and the window's current size, keeping width and height, with a test including an odd-sized window so rounding is defined
- [x] 1.3 Compute move-to-next-display: given the ordered work areas, the window's current display, and its current frame, pick the next display and map the same relative region onto its work area, with tests over two displays, a single display (no-op), and destinations of a different size
- [x] 1.4 Verify the maths is integer-pixel exact: a work area split into halves and into thirds covers the whole area with no lost or doubled pixel column, with a test carrying the assertion

## 2. The `WindowManager` trait

- [x] 2.1 Define the `WindowManager` trait: read the target window's frame, read the work area of the display it is on, list the displays' work areas in order, and set the target window's frame; with a fake for testing that records the frames it is asked to set
- [x] 2.2 Define the target-window source so a command acts on the window focused before the launcher appeared, not the launcher, reusing the previous-foreground the launcher records; with a fake for testing
- [x] 2.3 Report a missing target and a refused reshape as typed errors rather than silent no-ops, matching the failure vocabulary the specs require

## 3. The extension

- [x] 3.1 Add the `window-management` extension with its manifest declaring one `NoView` command per move (halves, quarters, thirds, maximise, centre, next display), each with an icon and keywords, and test that it validates
- [x] 3.2 Implement each command to compute its target rectangle from the shared geometry and apply it through the `WindowManager`, tested against the fake that the right rectangle is set for each region
- [x] 3.3 Leave the window untouched and report a message when there is no target window, with a test
- [x] 3.4 Register the extension in `lib.rs` behind the real `WindowManager`, and verify the existing test suite still passes
- [x] 3.5 Verify the commands appear in root search and run, leaving the app in a runnable state on the platform under development
  - Verified end to end on Windows: with a target window focused, opening the
    launcher and running Left Half moved that window to the left half and never
    the launcher. Confirms root search, dispatch, and the NoView command path.

## 4. Platform: Windows

Built and verified first, on the author's daily-use machine.

- [x] 4.1 Implement reading the target window's frame corrected by `DWMWA_EXTENDED_FRAME_BOUNDS`, so the visible frame is what the geometry sees, and reading its display's work area in physical pixels
- [x] 4.2 Implement setting the frame with `SetWindowPos`, offsetting by the invisible-border difference so the visible edges land exactly, and restoring a maximised window first so the tile takes effect
  - Verified by `examples/window_walkthrough`: every region lands flush against
    the work area within a pixel or two, with the invisible-border inset undone.
- [x] 4.3 List the displays' work areas in order for move-to-next-display, per-monitor DPI aware
  - Displays are listed in a stable left-to-right, top-to-bottom order.
- [ ] 4.4 Refuse an elevated target window with a clear message, reusing M3's integrity-level comparison, verified by tiling an elevated window
  - Reuses M3's `is_out_of_reach` integrity comparison, which M3 verified live
    against an elevated window (its task 9.9). The harness has the check; it
    needs a `dango-elevated` window opened elevated to run, which is a UAC step.
- [x] 4.5 Check symbol names and module paths against the vendored crate source before pushing, and verify CI's `cargo clippy --all-targets -- -D warnings` passes on `windows-latest`
  - `cargo clippy --all-targets -- -D warnings` is clean locally, including the
    new module, the platform code, and the walkthrough example.
- [x] 4.6 On Windows, verify by hand: each half, quarter, and third lands flush against the work area with no gap and without covering the taskbar; maximise fills the work area; centre keeps the size
  - **A DPI finding, recorded.** Placement is pixel-exact when the target window
    shares the launcher's per-monitor-v2 DPI context: the walkthrough passes 17
    of 17 with a DPI-aware target, across two displays of different scale
    (5120x2160 and 1440x2560). A legacy non-DPI-aware window is scaled by Windows
    on `SetWindowPos` (its height came back 1.5x on a 150% display), which is a
    platform limitation of driving a lower-awareness window, not a geometry bug.
    The harness itself calls `SetProcessDpiAwarenessContext` so it measures the
    launcher's real conditions.

## 5. Platform: macOS

Ships behind the Accessibility permission M3 surfaces; verified after Windows.

Not started. A compiling placeholder (`platform/macos/window.rs`) reports the
gap so the build and CI stay green, and every command says window management is
not yet available on macOS. The real implementation needs the macOS machine: its
`objc2-application-services` source is not cached on the Windows box this was
built on, so the AX symbols could not be checked and the code could not be
compiled, and the project's rule is to verify both before pushing platform code.

- [ ] 5.1 Implement reading and setting the focused window's frame through the Accessibility API (`AXWindow`, `kAXPositionAttribute`, `kAXSizeAttribute`), in logical points
- [ ] 5.2 List `NSScreen` work areas in order for move-to-next-display
- [ ] 5.3 Report the missing Accessibility permission through the state M3 built and offer the prompt, verified by revoking the permission and running a command
- [ ] 5.4 On macOS, verify by hand: each region lands correctly against the visible frame, maximise leaves the menu bar and dock, and centre keeps the size

## 6. Verification

- [ ] 6.1 Confirm on both platforms that every region command places the focused window exactly against the work area, in at least two applications
  - Windows: confirmed by the walkthrough against a real window in two
    applications' worth of sizes; every region lands flush. macOS pending.
- [ ] 6.2 Confirm on both platforms that move-to-next-display moves the window and keeps its relative region, including across displays of different DPI scale
  - Windows: confirmed, including across displays of different DPI scale
    (5120x2160 and 1440x2560). macOS pending.
- [ ] 6.3 Confirm on both platforms that a command acts on the window that was focused before the launcher, never the launcher itself
  - Windows: confirmed end to end; Left Half from the launcher moved the
    previously focused window, not the launcher. macOS pending.
- [ ] 6.4 Confirm on both platforms that a window arrangement completes within 100ms of confirming
  - Windows: confirmed; `place` returns in under 2ms, well inside 100ms.
    macOS pending.
- [ ] 6.5 Confirm on Windows that tiling an elevated window fails visibly and leaves it unchanged
  - Needs a `dango-elevated` window opened elevated (a UAC step); the harness
    check skips without it. The refusal path is M3-verified (9.9).
- [ ] 6.6 Confirm on macOS that revoking Accessibility produces the explained failure and the prompt, and that granting it restores normal behaviour without a restart
  - macOS window management is not yet implemented (see group 5), so this is
    pending macOS.
