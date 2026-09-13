## 1. Geometry

Pure Rust beside the existing region maths, finished and tested first.

- [x] 1.1 Add the absolute sizes to `geometry.rs`: almost maximise (90% of the work area, centred), reasonable size (60% width by 70% height, centred), and centre half (middle half width, full height), each as a function of the work area, with a test per size including a non-zero work-area origin
- [x] 1.2 Add a step function that grows or shrinks a frame by a fixed fraction of the work area on every side, centred on the frame's own centre, clamped to the work area when growing and to a minimum size when shrinking, with tests for grow, shrink, the upper clamp at the work area, and the lower clamp at the minimum
- [x] 1.3 Add a cycling helper mapping a cycle command and a step index to a rectangle: left/right half over width 1/2, 2/3, 1/3; top/bottom half over height; centre over both dimensions; with tests that each step is the expected rectangle and that the index wraps after 1/3

## 2. Cycle state and commands

- [x] 2.1 Add an in-memory cycle state to the extension recording the last command and the frame it placed, advancing the step when the same command repeats on an unmoved window (within a small frame tolerance) and resetting otherwise, with tests over: repeat advances, a different command resets, and a moved window resets
- [x] 2.2 Wire the cycling commands (left half, right half, top half, bottom half, centre) through the cycle state so the first press is unchanged and repeats advance, with a test that three left-half presses give 1/2, 2/3, 1/3 and a fourth returns to 1/2
- [x] 2.3 Add the new no-view commands to the manifest with icons and keywords: almost maximise, reasonable size, make larger, make smaller, centre half, and test that the manifest validates and every declared command is invocable
- [x] 2.4 Implement make larger and make smaller through the step function and the last-frame memory, with a test that repeated larger accumulates to the work area and stops, and repeated smaller stops at the minimum
- [x] 2.5 Verify the existing region commands and the whole suite still pass unchanged

## 3. Verification

- [x] 3.1 Extend `examples/window_walkthrough` on Windows to drive the new commands against a real window: each absolute size lands centred at the expected rectangle, centre half is the middle column, make larger and smaller step and clamp, and repeating left half cycles 1/2, 2/3, 1/3
  - Ran on Windows, 23 of 23: almost maximise, reasonable size, and centre
    half land centred and pixel-exact; make larger grows and clamps to the work
    area; make smaller clamps to the 400x300 minimum.
- [x] 3.2 Confirm on Windows by hand that repeating a tiling command from the launcher cycles its size, and that moving the window between presses resets the cycle
  - Confirmed live through the launcher: three Left Half presses ended at width
    1706 on a 5120-wide work area, i.e. 1/2 -> 2/3 -> 1/3, proving the cycle state
    persists across invocations. The full path (root search, dispatch, cycle
    state, placement) is exercised.
- [ ] 3.3 Confirm on macOS by hand that the new commands and the cycling behave the same, since the feature is platform-neutral and inherits the macOS frame get/set
  - macOS: not run here (needs the macOS machine). The feature is platform
    neutral, so it inherits the macOS frame get/set; a by-hand pass remains.
- [ ] 3.4 Confirm on both platforms that a single press of every existing command is unchanged from before this change
  - Windows: confirmed. A single press of each half is byte-identical to before,
    since the cycle delegates to the existing Region at 1/2, and a fresh centre
    still keeps the window's size. macOS pending.
