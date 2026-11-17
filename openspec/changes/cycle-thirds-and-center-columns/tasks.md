# Tasks

## 1. Cycle geometry

- [ ] 1.1 In `src/extensions/window_management/geometry.rs`, make the left and
  right rings of `cycle_rect` return tiling rectangles at every step, not only
  at 1/2: the 1/3 step equals `Region::LeftThird` / `Region::RightThird`, the
  left 2/3 ends where `Region::RightThird` starts, and the right 2/3 starts
  where `Region::LeftThird` ends. Top, bottom, and centre are untouched. Verify
  with geometry tests on an odd-sized work area (1001 x 769 at a non-zero
  origin) that assert those equalities and that left 2/3 plus right 1/3, and
  left 1/3 plus right 2/3, cover the width exactly
  (`cargo test window_management::geometry`).
- [ ] 1.2 Add one `Cycle` variant for the centred column: the 1/2 step equals
  `center_half`, the 1/3 step equals `Region::CenterThird`, the 2/3 step is 2/3
  of the width centred, all at full height. Keep `center_half`, which
  `examples/window_walkthrough.rs` also calls. Verify with geometry tests on the
  same odd-sized area for all three steps and the wrap.

## 2. Commands

- [ ] 2.1 In `src/extensions/window_management/mod.rs`, give
  `Arrangement::Cycle` a start step and map `left-third` and `right-third` to
  the left and right rings starting at 1/3, `center-third` to the centred
  column starting at 1/3, and `center-half` to it starting at 1/2. Remove the
  `Arrangement::CenterHalf` arm. Verify with `WindowSim` tests: four presses of
  each of the four commands place the expected ring and wrap to the command's
  first size.
- [ ] 2.2 Cover the cross-command rules with `WindowSim` tests: two presses of
  right third place `Region::RightHalf`'s rect, three presses of centre half
  place `Region::CenterThird`'s rect, a moved window restarts centre third at
  1/3, and left half after two presses of left third places the left 1/2.
  Verify with `cargo test window_management`.

## 3. Integration

- [ ] 3.1 Run `mise run check`, `cargo test`, and
  `cargo build --example window_walkthrough`; all pass.
- [ ] 3.2 Run the `ponytail-review` skill in a fresh subagent on this change's
  commits, without the apply context, and report its findings.

## Workflow follow-up

- Archive the change with `/opsx:archive` once the review findings are settled.
