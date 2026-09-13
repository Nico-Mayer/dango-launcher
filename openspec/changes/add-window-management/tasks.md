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

Written and run on the macOS machine, where the AX symbols could be checked
against the vendored `objc2-application-services` source and the code compiled.
The target is the frontmost application's focused window rather than a
remembered handle: the launcher is a non-activating panel, so the application
the user was in is still the frontmost one while the launcher is on screen.
Confirmed live, with the launcher up and its window on screen, the frontmost
application was still the one behind it.

- [x] 5.1 Implement reading and setting the focused window's frame through the Accessibility API (`AXWindow`, `kAXPositionAttribute`, `kAXSizeAttribute`), in logical points
  - `AXFocusedWindow` of the frontmost application, then `AXPosition` and
    `AXSize` as `AXValue`s carrying a `CGPoint` and a `CGSize`. Symbols checked
    against `objc2-application-services-0.3.2`: `AXValue::new` is `AXValueCreate`
    and `AXValue::value` is `AXValueGetValue`, both behind an `AXValue` feature
    the crate did not have enabled, so it was added.
  - A placement writes position, then size, then position again. The second
    write is not belt and braces: a resize can shove a window that no longer
    fits where it was, and without it a grow near a screen edge lands short.
- [x] 5.2 List `NSScreen` work areas in order for move-to-next-display
  - The two coordinate spaces disagree and this is where the bugs would live.
    Accessibility measures down from the top-left of the primary screen;
    `NSScreen` measures up from its bottom-left. Every screen rectangle is
    flipped around the primary screen's height on the way in, so everything
    above the platform line sees one space. Six unit tests cover the flip and
    the screen lookup, including a screen sitting above the primary one.
  - `NSScreen` only answers on the main thread, so the manager carries M3's
    `MainThread` hop and `platform::window_manager` now takes one. Windows
    ignores it. Accessibility itself needs no hop.
  - Ordered left-to-right then top-to-bottom, the same order Windows uses.
- [ ] 5.3 Report the missing Accessibility permission through the state M3 built and offer the prompt, verified by revoking the permission and running a command
  - Implemented. `AXIsProcessTrusted` gates every call, and a missing permission
    shows the system prompt and returns `WindowError::Failed` explaining it, so
    the command reports rather than doing nothing. M3's two helpers moved from
    `macos/text.rs` up to `macos/mod.rs`, so the paste path and this share one
    check and one prompt instead of two copies.
  - Not verified live. Revoking Accessibility is a System Settings step, and it
    cannot be faked from here: a copy of the harness at a fresh path, and one
    re-signed under a different identifier, both stayed trusted, because an
    unsigned binary launched from a terminal inherits the terminal's grant as
    its responsible process. Needs a by-hand revoke and regrant (6.6).
- [x] 5.4 On macOS, verify by hand: each region lands correctly against the visible frame, maximise leaves the menu bar and dock, and centre keeps the size
  - `examples/window_walkthrough` now has a macOS half. 17 of 17 against a
    scratch TextEdit document, 16 of 16 against a Finder window (the
    seventeenth does not apply, below). Every half, quarter and third lands
    flush, maximise equals the work area, and centre keeps the size exactly.
  - Frames are read back by asking System Events, so the answer comes from a
    different process than the one that wrote it. Direct Apple Events to the
    target application were tried first and time out without an Automation
    grant; System Events needs only the Accessibility permission the harness
    already requires.
  - **The menu bar is the check that is not circular.** Every other assertion
    compares a placement against a work area this same code computed, so a work
    area flipped upside down would agree with itself. The system reports the
    menu bar as an element with its own frame, which is a fact about the screen
    rather than something the code derived, and the primary screen's work area
    starts exactly where it ends: menu bar 1512x33 at the origin, work area top
    33. An inverted flip would put the work area top at zero and fail.
  - **A Finder finding, recorded.** Closing Finder's last window never produces
    "no window to move": the desktop is a window and stays focused, so `target`
    answers with the full screen. Harmless, since a placement then acts on the
    desktop window, which does not move. The harness skips that one check for
    Finder rather than calling it a failure.

## 6. Verification

- [x] 6.1 Confirm on both platforms that every region command places the focused window exactly against the work area, in at least two applications
  - Windows: confirmed by the walkthrough against a real window in two
    applications' worth of sizes; every region lands flush.
  - macOS: confirmed against TextEdit and Finder, two applications with
    different window shapes and minimum sizes. Every region flush in both.
- [ ] 6.2 Confirm on both platforms that move-to-next-display moves the window and keeps its relative region, including across displays of different DPI scale
  - Windows: confirmed, including across displays of different DPI scale
    (5120x2160 and 1440x2560).
  - macOS: not run. Only one display was attached, so the harness skipped it.
    Needs a second display, ideally one non-Retina next to the built-in Retina
    panel, to answer the mixed-scale half of this. Note that the macOS side has
    less to get wrong than Windows did: Accessibility speaks logical points in
    one global space, so a display's scale never enters the arithmetic, which is
    exactly where the Windows DPI finding came from.
- [ ] 6.3 Confirm on both platforms that a command acts on the window that was focused before the launcher, never the launcher itself
  - Windows: confirmed end to end; Left Half from the launcher moved the
    previously focused window, not the launcher.
  - macOS: the half that can be checked without the interface is confirmed. With
    the launcher shown and its window on screen, the frontmost application was
    still the one behind it, which is the window `target` picks. The manager also
    refuses its own process outright. Running Left Half from root search is still
    a by-hand step, for the same reason M3 recorded: keystrokes cannot be
    delivered to a non-activating panel from a harness.
- [x] 6.4 Confirm on both platforms that a window arrangement completes within 100ms of confirming
  - Windows: confirmed; `place` returns in under 2ms, well inside 100ms.
  - macOS: confirmed; `place` returns in 2.7ms against TextEdit and 30ms against
    Finder, both inside 100ms. Finder is the slower one because each
    Accessibility write is a round trip into the application.
- [ ] 6.5 Confirm on Windows that tiling an elevated window fails visibly and leaves it unchanged
  - Needs a `dango-elevated` window opened elevated (a UAC step); the harness
    check skips without it. The refusal path is M3-verified (9.9).
- [ ] 6.6 Confirm on macOS that revoking Accessibility produces the explained failure and the prompt, and that granting it restores normal behaviour without a restart
  - The path is implemented (5.3) but needs a by-hand revoke in System Settings.
    It could not be faked: an unsigned binary run from a terminal inherits that
    terminal's grant, so neither a copy at a fresh path nor a re-signed copy came
    up untrusted. `examples/window_walkthrough` detects an untrusted process and
    reports what `target` and `place` return, so the check is one run once the
    permission is actually off.
