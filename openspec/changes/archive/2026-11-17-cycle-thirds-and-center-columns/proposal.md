# Proposal

## Why

The half commands and Center cycle their size on a repeat, but Left Third,
Center Third, Right Third, and Center Half do nothing on a second press. The
repeat rule is the same idea for all of them, so the gap is surprising, and
there is no way to get a centred 2/3 column at all.

## What Changes

- Left Third, Right Third, Center Third, and Center Half become cycling
  commands. A repeat on the unmoved window steps the width through the ring the
  half commands already use (1/2, 2/3, 1/3), at full height.
- Each command enters the ring at its own fraction: the thirds at 1/3 (then 1/2,
  then 2/3), Center Half at 1/2 (then 2/3, then 1/3).
- The anchor never changes: Left and Right Third stay against their edge, Center
  Third and Center Half stay centred horizontally.
- A first press is unchanged for all four, so the three thirds still tile the
  work area with no gap.
- The same size reached from different commands is the same rectangle: Left
  Third and Left Half share one ring, Right Third and Right Half share one, and
  Center Third and Center Half share one. Each ring's sizes tile with the
  opposite side's, so a left 2/3 sits flush against a right 1/3.
- As a result, the 2/3 and 1/3 steps of Left Half and Right Half can move by up
  to 2 pixels on work areas whose width does not divide by 3. Their first press
  is unchanged.
- Moving the window or switching commands resets the cycle, as for the halves.

No user-visible text changes. No manifest or view protocol change.

Rejected while exploring:

- Position cycle for thirds (left, centre, right): repeats what the three
  separate third commands already do.
- Height cycle for the centred columns: repeats what Center does on a repeat.
- A separate ring per command: one ring with different entry points is one rule
  to remember, and it keeps equal sizes on equal pixels.

No design artifact: the change reuses the existing cycle mechanism and makes no
new architectural decision.

## Known limit

A window that refuses a width (its minimum is wider than the requested step)
does not advance: the next press sees a frame that differs from the one asked
for and restarts at the command's first size. For the thirds this is the first
step, so such a window stays at its minimum width against the anchor. The halves
have the same limit today at their 1/3 step. Not addressed here.

## Non-goals

- Cycling for the quarters, Maximize, Almost Maximize, or Reasonable Size.
- Changing the ring order, Top Half, Bottom Half, or Center.
- New commands (for example a dedicated two-thirds command).
- Configurable cycle sizes.
- Advancing the cycle on a window that refuses the requested size.

## Platforms

Windows and macOS. The change is pure geometry and shared cycle state, with no
platform code.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `window-management`: the third commands and Center Half gain a width cycle on
  repeat; the cycling requirement names their base results.

## Impact

- `src/extensions/window_management/geometry.rs`: one centred-column cycle, and
  the left and right rings return the tiling rectangles at every step.
- `src/extensions/window_management/mod.rs`: the four commands map to cycling
  arrangements with a start step; tests.
- `examples/window_walkthrough.rs` keeps using `geometry::center_half`.
