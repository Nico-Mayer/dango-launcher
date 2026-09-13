## 1. Shared geometry

Pure Rust with no platform and no UI, so it is finished and tested first and the
platform seam stays as small as possible.

- [ ] 1.1 Add a `window_management` module with a `Region` for the fixed moves (left/right/top/bottom halves, four quarters, left/centre/right thirds, maximise) and a function mapping a `Region` plus a work-area rectangle to a target rectangle, with tests over each region including that halves and quarters tile without gaps or overlap
- [ ] 1.2 Compute centre from a work area and the window's current size, keeping width and height, with a test including an odd-sized window so rounding is defined
- [ ] 1.3 Compute move-to-next-display: given the ordered work areas, the window's current display, and its current frame, pick the next display and map the same relative region onto its work area, with tests over two displays, a single display (no-op), and destinations of a different size
- [ ] 1.4 Verify the maths is integer-pixel exact: a work area split into halves and into thirds covers the whole area with no lost or doubled pixel column, with a test carrying the assertion

## 2. The `WindowManager` trait

- [ ] 2.1 Define the `WindowManager` trait: read the target window's frame, read the work area of the display it is on, list the displays' work areas in order, and set the target window's frame; with a fake for testing that records the frames it is asked to set
- [ ] 2.2 Define the target-window source so a command acts on the window focused before the launcher appeared, not the launcher, reusing the previous-foreground the launcher records; with a fake for testing
- [ ] 2.3 Report a missing target and a refused reshape as typed errors rather than silent no-ops, matching the failure vocabulary the specs require

## 3. The extension

- [ ] 3.1 Add the `window-management` extension with its manifest declaring one `NoView` command per move (halves, quarters, thirds, maximise, centre, next display), each with an icon and keywords, and test that it validates
- [ ] 3.2 Implement each command to compute its target rectangle from the shared geometry and apply it through the `WindowManager`, tested against the fake that the right rectangle is set for each region
- [ ] 3.3 Leave the window untouched and report a message when there is no target window, with a test
- [ ] 3.4 Register the extension in `lib.rs` behind the real `WindowManager`, and verify the existing test suite still passes
- [ ] 3.5 Verify the commands appear in root search and run, leaving the app in a runnable state on the platform under development

## 4. Platform: Windows

Built and verified first, on the author's daily-use machine.

- [ ] 4.1 Implement reading the target window's frame corrected by `DWMWA_EXTENDED_FRAME_BOUNDS`, so the visible frame is what the geometry sees, and reading its display's work area in physical pixels
- [ ] 4.2 Implement setting the frame with `SetWindowPos`, offsetting by the invisible-border difference so the visible edges land exactly, and restoring a maximised window first so the tile takes effect
- [ ] 4.3 List the displays' work areas in order for move-to-next-display, per-monitor DPI aware
- [ ] 4.4 Refuse an elevated target window with a clear message, reusing M3's integrity-level comparison, verified by tiling an elevated window
- [ ] 4.5 Check symbol names and module paths against the vendored crate source before pushing, and verify CI's `cargo clippy --all-targets -- -D warnings` passes on `windows-latest`
- [ ] 4.6 On Windows, verify by hand: each half, quarter, and third lands flush against the work area with no gap and without covering the taskbar; maximise fills the work area; centre keeps the size

## 5. Platform: macOS

Ships behind the Accessibility permission M3 surfaces; verified after Windows.

- [ ] 5.1 Implement reading and setting the focused window's frame through the Accessibility API (`AXWindow`, `kAXPositionAttribute`, `kAXSizeAttribute`), in logical points
- [ ] 5.2 List `NSScreen` work areas in order for move-to-next-display
- [ ] 5.3 Report the missing Accessibility permission through the state M3 built and offer the prompt, verified by revoking the permission and running a command
- [ ] 5.4 On macOS, verify by hand: each region lands correctly against the visible frame, maximise leaves the menu bar and dock, and centre keeps the size

## 6. Verification

- [ ] 6.1 Confirm on both platforms that every region command places the focused window exactly against the work area, in at least two applications
- [ ] 6.2 Confirm on both platforms that move-to-next-display moves the window and keeps its relative region, including across displays of different DPI scale
- [ ] 6.3 Confirm on both platforms that a command acts on the window that was focused before the launcher, never the launcher itself
- [ ] 6.4 Confirm on both platforms that a window arrangement completes within 100ms of confirming
- [ ] 6.5 Confirm on Windows that tiling an elevated window fails visibly and leaves it unchanged
- [ ] 6.6 Confirm on macOS that revoking Accessibility produces the explained failure and the prompt, and that granting it restores normal behaviour without a restart
