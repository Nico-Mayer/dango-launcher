## Context

See proposal.md for why. What already exists and shapes this:

- `extensions/window_management/geometry.rs` turns a `Region` plus a work-area
  `Rect` into a target rectangle, and has `center` and `next_display`. It is pure
  and unit tested. The new sizes and the step maths belong here beside them.
- `extensions/window_management/mod.rs` declares one no-view command per move and
  an `Arrange { manager, arrangement }` command that reads `target()`, computes a
  rectangle, and calls `place()`. New commands are new `Arrangement` variants.
- The `WindowManager` trait returns a `Placement { frame, work_area }` from
  `target()`. That is everything the new work needs: the current frame for the
  relative moves, the work area for the absolute ones. The trait does not change.
- The extension is a single struct today with no state. The cycle needs a little.

## Goals / Non-Goals

**Goals:**

- Add the sizes and the cycling with zero platform code, so both platforms get it
  the moment this lands and the seam that cannot be compiled locally is untouched.
- Keep the first press of every existing command byte-for-byte what it is now.

**Non-Goals:**

- See proposal.md. At design level additionally: no persistence of the cycle
  across restarts, and no undo. The cycle is in-memory and best-effort.

## Decisions

### The cycle keys on the last command and the frame it produced, not a window id

The cycle needs to know "is this the same command acting on the same window,
still where I left it". The obvious key is a window identity, and an earlier
sketch added one to `Placement`.

It is dropped, on two grounds. macOS has no stable window identity through the
public Accessibility API; a real id there needs the private
`_AXUIElementGetWindow` SPI, which is exactly the kind of unsupported dependency
the project avoids, and adding a field to `Placement` would mean editing the
freshly landed, locally uncompilable macOS code to populate it. And the cheaper
key is good enough: remember the last command and the exact frame it placed, and
on the next invocation advance the cycle only if the same command is repeated and
the window is still at that frame. Anything else, including the user moving the
window or switching windows, resets to the first size.

The one case this key gets wrong is two different windows sitting at pixel-identical
frames, where repeating the command on the second advances instead of restarting.
It is rare, the harm is one wrong size, and the next press corrects it. Worth it
to keep the change platform-free.

```
invoke(cmd):
  p = target()
  if cmd == last.cmd and p.frame ~= last.placed_frame:
      step = (last.step + 1) mod N          # same command, window unmoved
  else:
      step = 0                              # fresh
  frame = size_for(cmd, step, p)
  place(frame)
  last = { cmd, placed_frame: frame, step }
```

A small tolerance on the frame match absorbs the border rounding the platforms
already have; an exact match would reset spuriously after a pixel-off placement.

Centre is the one command whose reset state is not a cycle member. M4 shipped
centre as "keep the window's size, centred", and that single-press behaviour must
not regress. So a fresh centre keeps size as before, and only a repeat on the
unmoved window enters the 1/2, 2/3, 1/3 cycle. The half commands have no such
split: their reset state, 1/2, is already the first cycle member, so their
single press is unchanged for free.

### Make larger and smaller share the cycle's "unmoved" check, but step instead of cycle

Growing and shrinking read the current frame and adjust it, so they have the same
question the cycle does: is the window still where I last put it. They reuse the
same last-frame memory. The difference is they step a fraction of the work area on
every side rather than choosing from a fixed list, and they clamp: larger never
exceeds the work area, smaller never goes below a minimum. Because they clamp
rather than wrap, repeated presses accumulate to the limit and stop, which is what
the spec's "repeated steps accumulate" scenario asks for.

Stepping around the window's own centre, not its origin, keeps a centred window
centred and feels like zooming rather than dragging a corner.

### The new absolute sizes are fixed fractions, centred

- Almost maximise: the work area inset to 90% of its size, centred. A margin that
  reads as "nearly full" without being maximise.
- Reasonable size: 60% of the work area's width by 70% of its height, centred. A
  comfortable window for reading or writing.
- Centre half: the middle half of the width at full height, a centred column.

These are constants, computed the same way `center` already is. Making them
preferences is deferred to M7; hardcoding now keeps the change small and the
numbers live in one place to change later.

Rejected: deriving "reasonable" from the window's content or the display's
physical size. There is no signal for content size, and a fixed fraction of the
work area already scales with the display.

### The cycle set is fixed at 1/2, 2/3, 1/3

The order is deliberate: the common case is halving, then the two-thirds "primary
pane" split, then the one-third "side pane". A press cycles forward through them;
there is no reverse command, matching the tools this imitates. Making the set or
its order configurable is an M7 preference, not this change.

### No UI, no protocol change

Every new command is a no-view action, like the existing ones, so there is no
Bits UI primitive to name and no view protocol surface. `manifestVersion` and
`protocolVersion` do not move; this is additive at the extension level only.

## Risks / Trade-offs

- **The cycle state is global and best-effort.** Two windows at identical frames
  can share a cycle step. → Accepted and stated; the harm is one wrong size and
  self-corrects. The alternative, a private macOS SPI for window identity, costs
  more than the bug.
- **The "unmoved" check depends on a frame tolerance.** Too tight resets after a
  pixel-off placement; too loose treats a small user nudge as unmoved. → The
  tolerance matches the border rounding the platforms already produce, a few
  pixels, verified against real placements in the walkthrough.
- **Make larger or smaller on a window that refuses arbitrary sizes.** A window
  with a fixed minimum will not shrink past it. → The same ceiling every window
  tool has; the command applies the frame and the window keeps what it can.
