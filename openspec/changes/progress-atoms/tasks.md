# Tasks

## 1. Progress and status atoms

- [ ] 1.1 Add `ui/atoms/progress.slint` with `Progress` (`kind`: bar, ring, spinner; `value`: float, below 0 = indeterminate), drawn as design.md describes: bar track in `border`, fill in `working`, and indeterminate bar segment in `accent`; ring 16 px `Path` arc in `working` over a `border` track; spinner = indeterminate ring at the given size. Set `accessible-role: progress-indicator`, `accessible-value` 0 to 100 when determinate, and the label "Working" when indeterminate. Add each kind to `AtomGallery`. Verify with a UI test in `src/slint_shell/tests.rs` that finds a determinate ring at 0.42 with value 42 and an indeterminate one labelled "Working".
- [ ] 1.2 Rework `ui/atoms/dot.slint` to take `status` (working, blocked, done, failed, queued), coloured `working`, `blocked`, `done`, `danger`, `idle`, labelled "Working", "Needs input", "Done", "Failed", "Queued". Drop its `ToastStyle` import, and keep `ToastStyle` exported for the toast. Verify with a gallery UI test that each status's dot has its label.
- [ ] 1.3 In `ui/shell/action-bar.slint`, show `Progress { kind: spinner }` for a busy toast and `Dot` with `done` or `failed` for success and failure. In `ui/shell/hud.slint`, map success to `done` and failure to `failed`. Update the toast and HUD UI tests in `src/slint_shell/tests.rs` that assert the dot. Verify with `cargo test`: a busy toast exposes a progress indicator labelled "Working", and success and failure still expose their dot.
- [ ] 1.4 Replace the inline sweep in `ui/shell/search-bar.slint` with `Progress { kind: bar; value: -1; }`, keeping its height, accent segment, and the "Loading…" accessible label. Verify with the existing loading-bar UI tests in `cargo test`.

## 2. Progress and status accessories in the protocol

- [ ] 2.1 In `src/protocol.rs`, add `Accessory::Progress { value: Option<f32> }` and `Accessory::Status { status: Status }`, with `Status` = working, needsInput, done, failed, queued (camelCase on the wire). Verify with serde round-trip tests in `src/protocol.rs` for `{"kind":"progress","value":0.42}`, `{"kind":"progress"}`, and `{"kind":"status","status":"needsInput"}`.
- [ ] 2.2 Extend the refusal check in `src/ui/state.rs` (`refused`) to reject a progress value outside 0 to 1, through the same error-in-place-of-the-view path as invalid shortcuts. Verify with a reducer test in `src/ui/tests.rs` next to `assert_refused`: 1.5 is refused, and 0, 0.42, and 1 are not.
- [ ] 2.3 Add `progress` and `status` to `AccessoryKind`, and `value` and `status` to `AccessoryData`, in `ui/atoms/accessory.slint`, rendering `Progress { kind: ring }` and `Dot`. Map the new variants in `src/slint_shell/render/list.rs`. Verify with a UI test: a row with a ring at 0.42 and status working shows a progress indicator at 42 and a dot labelled "Working".

## 3. Loading marks that don't blink

- [ ] 3.1 In `src/ui/`, add the loading-mark state machine from design.md (Hidden, Pending, Shown, Lingering), `Effect::WakeLoadingMarks { id, after }`, and `Event::LoadingMarksDue { id }` and `Event::LoadingMarksHeld { id }`. Make the snapshot's `busy` and the "Loading…" toast follow `Shown` or `Lingering`, not the view's `loading` flag. Leaving the view or a reset goes to `Hidden` and bumps the id. Verify with reducer tests in `src/ui/tests.rs` for each spec scenario: fast load shows nothing; marks appear on `Due`; marks linger until `Held`; leaving clears at once; a stale id is ignored. Update existing tests that expect marks the moment loading starts by sending `LoadingMarksDue`.
- [ ] 3.2 In `src/app/mod.rs`, add `LOADING_MARK_DELAY` (150 ms) and `LOADING_MARK_HOLD` (400 ms), and run `WakeLoadingMarks` like `ExpireToast`: sleep, then apply the matching event. Verify with an app test in `src/app/tests.rs` that a view loading for longer than the delay gets `busy` in the snapshot, and one that finishes sooner never does.

## 4. Skeleton rows

- [ ] 4.1 Add `ui/atoms/skeleton.slint` (`rows`: int, each `Theme.row-height` high, an icon square and a title bar in `raised`, children `accessible-role: none`). Add it to `AtomGallery`. Verify with `cargo test` (gallery compiles and builds).
- [ ] 4.2 Give the snapshot a `skeleton` flag for a list view marked loading with no items, in place of the empty state. Set `ListData.skeleton` in `src/slint_shell/render/list.rs`, draw `Skeleton` in `ui/views/list.slint`, and in `src/slint_shell/render/height.rs` count the skeleton's fixed row count so the window takes its maximum height. Verify with UI tests: a loading empty list shows the skeleton and no empty state at full height, and the first items replace it in one render.

## 5. Docs and review

- [ ] 5.1 Check `AGENTS.md` for statements that assume nothing animates, and update them if any. Verify by `rg -n -i 'animat' AGENTS.md` showing no stale rule.
- [ ] 5.2 Run `mise run check` and `cargo test`. Verify both pass.
- [ ] 5.3 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context. Report the findings to the author. Verify that the findings are listed in the session.

## Workflow follow-up

- Change 2 of the design-system v2 stack. Stack its branch on `theme-graphite` with `git-spice branch create progress-atoms`.
- Archive after review with `/opsx:archive`.
