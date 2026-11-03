# Tasks

## 1. Progress view in the protocol and the UI

- [x] 1.1 In `src/protocol.rs`, add `View::Progress(ProgressView { title, subtitle, value: Option<f32>, detail, steps: Vec<Step { title, status: Status }>, actions })`, reusing `Status`. In `src/ui/state.rs`, add `Mode::Progress`, modelled on `Mode::Detail`: the surface takes focus; the bindings are Escape, Enter (primary action), and the panel chord; the panel works as on a detail; `is_loading` is false. Handle the new variant everywhere `View` is matched. Verify with a serde round-trip test in `src/protocol.rs`, and reducer tests in `src/ui/tests.rs`: Enter runs the progress view's primary action, and Escape pops it.
- [x] 1.2 Add `ScreenKind.progress`, a `ProgressData` global (title, subtitle, value, detail, steps), and `ui/views/progress-view.slint` laid out as the design-system delta says: title + muted subtitle, a 4 px `Progress { kind: bar }`, muted detail, and step rows with an unannounced `Dot` before each title. Queued titles are muted, and each row's accessible label is "<title>, <status word>". Add `ViewBody::Progress` to the snapshot, and `src/slint_shell/render/progress.rs` as the global's only writer (clamp the value, -1 for none). Send progress views to the `MAX` arm in `render/height.rs`. Verify with UI tests in `src/slint_shell/tests.rs`:
  - a progress view at 0.62 with three steps shows a bar reporting 62, the steps labelled like "Download, working", and the title and subtitle;
  - one without a value shows an indeterminate bar.
- [x] 1.3 Add `ActionOutcome::Back` in `src/extension/mod.rs`, handled in `action_finished` like Escape: pop one view and abandon the invocation. Verify with a reducer test: an action finishing with `Back` on a pushed view shows the view below, and a later render from that invocation is dropped.

## 2. Toast actions

- [x] 2.1 In `src/ui/`, give `Toast` and `HeldToast` an optional `ToastAction { title, shortcut, request: ActionRequest }`. While a toast with an action shows and the panel is closed, add its chord to `bindings`. In `Launcher::intent`, check that chord before `clear_failure`: a match emits `Effect::RunAction`, clears the toast, and the key does nothing else. Verify with reducer tests in `src/ui/tests.rs`:
  - the chord runs the action once and clears the toast;
  - another key clears the failure toast without running it;
  - the chord does nothing once the toast is gone;
  - a toast chord beats a root item's action with the same chord (Ctrl+R);
  - with the panel open, the chord is not bound.
- [x] 2.2 In `ui/shell/action-bar.slint`, draw the toast action's label and `Kbd` after the toast text, with the label and chord in the toast's accessible description. Extend the `Shell` global and `render/shell.rs`. Verify with a UI test: a failure toast with "Retry" shows "Retry" and the platform's chord label, and its description names them.

## 3. Task registry

- [x] 3.1 Add `src/tasks.rs` with `Tasks`, `TaskHandle`, `TaskState`, `TaskStatus`, and `progress_view(&TaskState, actions)`, as design.md describes:
  - an oldest-first `Vec`;
  - a `Weak<Core>` set once;
  - reports applied as `Event::Task` outside the lock;
  - a `watch` cancel signal;
  - `Done`, `Failed`, and `Cancelled` dropped after their report, `NeedsInput` kept.

  Put `Tasks` in `Services` and link it to Core where `ViewRunner` gets its `Weak<Core>`. Verify with unit tests in `src/tasks.rs` (cancel flips the receiver; ended tasks leave; `NeedsInput` stays; `progress_view` maps steps and value) and an app test in `src/app/tests.rs` (a report reaches the reducer's snapshot).
- [x] 3.2 In the reducer:
  - Keep the last `TaskState` per id.
  - Make the oldest `Running` task the third busy source while its owner's progress view is not on top, with its Cancel as a toast action.
  - Compute `ViewScreen.busy` as `marks shown && (working || top view loading)`.
  - Results: hidden means `pending_result` plus the HUD for a failure; shown with the owner's view on top means nothing more; shown otherwise means a toast at once.
  - On `Activated`, turn `pending_result` into a toast unless the owner's progress view is on top.
  - Attach Retry to failure toasts when the task names it.

  Verify with reducer tests for each scenario in the `background-tasks` delta and the new design-system busy-mark scenario:
  - background toast at root, oldest of two;
  - no busy toast for `NeedsInput`;
  - the Cancel chord runs the owner's cancel action;
  - no loading bar in another view while a task runs;
  - failed while hidden: HUD now and toast on open;
  - ready while hidden: success toast on open;
  - reopening onto the task's own view: no toast;
  - ready while at root: toast at once.

## 4. The update on a task

- [ ] 4.1 Rewrite the install flow in `src/extensions/updates/mod.rs` on a task, as the design's flow shows:
  - steps "Download", "Verify signature", "Restart";
  - download progress throttled by `progress_step`;
  - on success, keep the `Update` and the bytes in `Inner::ready`, with the "Restart" step needing input and the detail "Ready to restart";
  - a download error fails "Download" and a signature error fails "Verify signature", each with its sentence and retry "install";
  - `select!` on the cancel signal, reporting `Cancelled` and answering `Back`;
  - "Restart now" installs the kept update.

  Skip scheduled checks while a task runs or waits ready. Remove `installing` and `shown`. Keep `task: Option<TaskHandle>`, so "Install update" while running shows the view again. Change the source's `install` to take the kept update. Verify with tests in `src/extensions/updates/mod.rs` using the fake source:
  - step order and statuses;
  - ready waits without installing;
  - "Restart now" installs once and uses the kept update after a newer check;
  - cancel reports `Cancelled` and installs nothing;
  - download and signature errors each report their sentence with Retry;
  - no scheduled check while ready.
- [ ] 4.2 Make the update item and the "Check for Updates" result follow the state:
  - "Install update" when idle;
  - "Show progress" while running;
  - when ready: the title "Dango 0.5.0 is ready to restart" with "Restart now".

  Give the progress view the actions from the design's table, with "Continue in background" and "Install later" answering `Back`. Verify with tests in `src/extensions/updates/mod.rs` for each item state, the check result's label, and "Install later".

## 5. Checks and review

- [ ] 5.1 Run `mise run check` and `cargo test`. Verify both pass.
- [ ] 5.2 macOS hand check with the real keyboard, since winit delivers Cmd+. through `cancelOperation:` and `dango-ui` `press` skips winit: while an update downloads in a release build, press Cmd+. on its busy toast and on its progress view, and confirm the download is cancelled; press Cmd+R on a Retry toast and confirm the download restarts. Leave open until a session on the Mac with an update to install.
- [ ] 5.3 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context. Report the findings to the author. Verify that the findings are listed in the session.

## Workflow follow-up

- Change 3 of the design-system v2 stack, on branch `task-registry` (stacked on `progress-atoms` with git-spice).
- The install step itself (NSIS on Windows, bundle swap on macOS) does not change, only when it runs. A real update is tried with the next release.
- Don't archive while 5.2 is open (AGENTS.md).
