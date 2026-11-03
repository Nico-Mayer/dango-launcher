# Tasks

## 1. Progress and status atoms

- [ ] 1.1 Add `ui/atoms/progress.slint` with `Progress` (`kind`: bar, ring; `value`: float, below 0 = indeterminate; `size` for rings; `track` and `fill` colours defaulting to `Theme.raised` and `Theme.working`). Draw the ring as design.md describes: clamp to `min(0.9999, value)`, `MoveTo` + `LineTo` start, no arc at 0, and indeterminate = quarter arc turned with `transform-rotation` from `animation-tick()`. Set `accessible-role: progress-indicator`. Determinate sets `accessible-value: round(value * 100)` with minimum 0 and maximum 100. Indeterminate sets the label "Working". Add a determinate ring, an indeterminate ring, and a determinate bar to `AtomGallery`. Verify with UI tests in `src/slint_shell/tests.rs`: the ring at 0.42 reports value 42, the indeterminate ring is labelled "Working", and a ring at 1 builds.
- [ ] 1.2 Move `ToastStyle` from `ui/atoms/dot.slint` to `ui/globals.slint`, and update its imports in `ui/shell/action-bar.slint`, `ui/shell/hud.slint`, and `ui/atoms/gallery.slint`. Rework `Dot` to take `status` (working, needs-input, done, failed, queued), coloured `working`, `blocked`, `done`, `danger`, `idle`, and an `announce` flag. When it is set, the label is "Working", "Needs input", "Done", "Failed", or "Queued". Without it, there is no label. Update the stale "never animated" comment. Verify with a gallery UI test that each announced status has its label.
- [ ] 1.3 In `ui/shell/action-bar.slint`, show a 12 px indeterminate `Progress { kind: ring }` for a busy toast, and a `Dot` (not announced) with `done` or `failed` for success and failure. In `ui/shell/hud.slint`, map success to `done` and failure to `failed`. Update the toast and HUD UI tests in `src/slint_shell/tests.rs` that assert the dot style, including the busy toast test around line 516. Verify with `cargo test`: a busy toast exposes a progress indicator labelled "Working", and success and failure toasts read as before.
- [ ] 1.4 Replace the inline sweep in `ui/shell/search-bar.slint` with `Progress { kind: bar; value: -1; track: Theme.border; fill: Theme.accent; }` at today's 2 px height, keeping the "Loading…" accessible label. Remove the "The only motion in the interface" comment. Update `the_loading_bar_sweeps_while_loading` (it finds `SearchBar::sweep` by element id) to the sweep's new id. Verify with the loading-bar UI tests in `cargo test`.

## 2. Progress and status accessories in the protocol

- [ ] 2.1 In `src/protocol.rs`, add `Accessory::Progress { value: Option<f32> }` and `Accessory::Status { status: Status }`, with `Status` = working, needsInput, done, failed, queued (camelCase on the wire). Verify with serde round-trip tests in `src/protocol.rs` for `{"kind":"progress","value":0.42}`, `{"kind":"progress"}`, and `{"kind":"status","status":"needsInput"}`.
- [ ] 2.2 Add `progress` and `status` to `AccessoryKind`, and `value` and `status` to `AccessoryData`, in `ui/atoms/accessory.slint`, rendering a 16 px `Progress { kind: ring }` and an announced `Dot`. Map the new variants in `src/slint_shell/render/list.rs`, clamping a progress value to 0 to 1 and sending -1 for no value. Verify with UI tests: a row with a ring at 0.42 and status working shows a progress indicator at 42 and a dot labelled "Working", and a ring at 1.5 reports 100 with the view shown as usual.

## 3. Busy marks that don't blink

- [ ] 3.1 In `src/ui/`, add the busy-mark state from design.md: `Hidden | Pending{id} | Shown{id, held}`, fed by `working.is_some() || top view loading`, a new id on every entry into `Pending`, `Effect::WakeBusyMarks { id, stage }` (`Delay`, `Hold`), and `Event::BusyMarksWoke { id }`. Make `ViewScreen.busy` and the busy toast (both "Running X…" and "Loading…") follow the marks. Delete `RootScreen.busy`. Verify with reducer tests in `src/ui/tests.rs` for each scenario in the "Busy marks do not blink" requirement:
  - a fast command shows no toast;
  - "Running X…" passes to "Loading…" without a gap;
  - marks stay until the hold wake;
  - a streamed loading view keeps the same id and state;
  - leaving clears at once;
  - a stale id is ignored.
- [ ] 3.2 Update the existing tests that expect busy marks the moment work starts, by sending `BusyMarksWoke` where marks are expected: in `src/ui/tests.rs`, tests using the `detail_tree` fixture (`loading: true`, around line 75) whose effect lists now include `WakeBusyMarks`, the busy test around line 835, and the `root_screen().busy` test around line 1823 (move it to the toast); in `src/slint_shell/tests.rs`, the loading-bar tests around lines 393 and 410 and the busy-toast test. Verify with `cargo test`.
- [ ] 3.3 In `src/app/mod.rs`, add `BUSY_MARK_DELAY` (150 ms) and `BUSY_MARK_HOLD` (400 ms), and run `WakeBusyMarks` like `ExpireToast`: pick the duration by `stage`, sleep, and apply `BusyMarksWoke`. Verify with an app test in `src/app/tests.rs`: a view that stays loading past the delay gets busy marks, and one that finishes sooner never does.

## 4. Skeleton rows

- [ ] 4.1 Add `ui/atoms/skeleton.slint` (`Skeleton` fills its height with `floor(height / Theme.row-height)` rows, each with an icon square and a title bar in `Theme.raised`, children `accessible-role: none`). Add it to `AtomGallery`. Verify with `cargo test` (gallery compiles and builds).
- [ ] 4.2 Give the snapshot's list body a `skeleton` flag when the view is loading and its declared items are empty, in place of the empty state. Do not set it when the query filters every item out. Set `ListData.skeleton` in `src/slint_shell/render/list.rs`, and set it to false on the root path. Draw `Skeleton` in `ui/views/list.slint`. In `src/slint_shell/render/height.rs`, let a skeleton list fall into the `_ => MAX` arm. Verify with UI tests: a loading list with no items shows the skeleton at maximum height and no empty state; the first items replace it in one render; a loading list with items filtered to nothing shows no skeleton.

## 5. Checks and review

- [ ] 5.1 Run `mise run check` and `cargo test`. Verify both pass.
- [ ] 5.2 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context. Report the findings to the author. Verify that the findings are listed in the session.

## Workflow follow-up

- Change 2 of the design-system v2 stack, on branch `progress-atoms` (stacked on `theme-graphite` with git-spice).
- Archive after review with `/opsx:archive`.
