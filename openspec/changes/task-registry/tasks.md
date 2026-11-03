# Tasks

## 1. Progress view in the protocol and the UI

- [ ] 1.1 In `src/protocol.rs`, add `View::Progress(ProgressView { title, subtitle, value: Option<f32>, detail, steps: Vec<Step { title, status: Status }>, actions })`, reusing `Status`. Handle the new variant wherever `View` is matched (`src/ui/state.rs` `is_loading` → false, actions, focus). Verify with a serde round-trip test in `src/protocol.rs` for a progress view with steps and no value.
- [ ] 1.2 Add `ScreenKind.progress`, a `ProgressData` global (title, subtitle, value, detail, steps), and `ui/views/progress-view.slint` laid out as the design-system delta says: title + muted subtitle, a 4 px `Progress { kind: bar }`, muted detail, and step rows with a `Dot` before each title. Queued titles are muted. Each step row's accessible label is "<title>, <status word>". Add `ViewBody::Progress` to the snapshot, `src/slint_shell/render/progress.rs` as the global's only writer (clamp the value, -1 for none), and send progress views to the `MAX` arm in `render/height.rs`. Verify with UI tests in `src/slint_shell/tests.rs`: a progress view at 0.62 with four steps shows a bar reporting 62, the four steps with labels like "Download, working", and the title and subtitle; one without a value shows an indeterminate bar.

## 2. Toast actions

- [ ] 2.1 In `src/ui/`, give `Toast` and `HeldToast` an optional `ToastAction { title, shortcut, run: ToastRun }` with `ToastRun::Action(ActionRequest)` and `ToastRun::CancelTask(u64)`, plus `Effect::CancelTask { id }`. While a toast with an action shows, add its chord to `bindings`, and handle it in the key path before the view's bindings: run it once, clear the toast, and let the key do nothing else. Verify with reducer tests in `src/ui/tests.rs`: the chord runs `RunAction` once and clears the toast; another key clears the failure toast without running it; the chord does nothing once the toast is gone; a toast action chord beats a root item's action with the same chord.
- [ ] 2.2 In `ui/shell/action-bar.slint`, draw the toast action's label and `Kbd` after the toast text, with the label and chord in the toast's accessible description. Extend the `Shell` global and `render/shell.rs`. Verify with a UI test: a failure toast with "Retry" shows "Retry" and the platform's chord label, and the description names it.

## 3. Task registry

- [ ] 3.1 Add `src/tasks.rs` with `Tasks`, `TaskHandle`, `TaskState`, `TaskStatus`, and `progress_view(&TaskState, actions)`, as design.md describes: start-by-key that refuses a second running task with the same key, report through `notify`, a `watch` cancel signal, and removal on `Done`, `Failed`, or `Cancelled`. Verify with unit tests in `src/tasks.rs`: a second start with the same key returns the running id; cancel flips the receiver; ended tasks leave the registry; `progress_view` maps steps and value.
- [ ] 3.2 Put `Tasks` in `Services` (`src/app/mod.rs`). Forward each change to the reducer as `Event::Task(TaskState)` on the main thread, and run `Effect::CancelTask` against the registry. Verify with an app test in `src/app/tests.rs`: a started task's report reaches the snapshot, and `CancelTask` flips its cancel signal.
- [ ] 3.3 In the reducer, keep task summaries. Add the oldest running task as the third busy source, only while the top view is not a progress view, with a Cancel toast action (`{ key: ".", modifiers: [Cmd] }`). Make the busy marks remember their source, and keep `ViewScreen.busy` false for a task source. Hold a task result that ends while hidden and show it on `Activated`: success expires, failure stays, and Retry is attached when the task named one. A result that ends while shown, with the top view not a progress view of the same owner, shows at once. Verify with reducer tests for each scenario in the `background-tasks` delta (background toast at root, oldest of two, Cancel chord emits `CancelTask`, failed while hidden shows on open, ready while hidden shows success, no loading bar in another view while a task runs).

## 4. The update on a task

- [ ] 4.1 In `src/extensions/updates/source.rs`, give `UpdateSource::download` an `on_finished` callback and pass it as `on_download_finish` in the production source. Update the fake source in the same file. Verify with `cargo test` (the fake calls `on_finished` before returning).
- [ ] 4.2 Rewrite the install flow in `src/extensions/updates/mod.rs` on a task, as the design's flow shows:
  - the four steps, with the download step's progress and the "Checking the signature" detail after `on_finished`;
  - "Ready to restart" with the bytes kept;
  - `select!` on the cancel signal;
  - a failure with Retry ("Install update");
  - the progress view pushed through the invocation context on each report.

  Remove `installing` and `shown`. Running "Install update" while the task runs shows its view again. Verify with tests in `src/extensions/updates/mod.rs` using the fake source: the step order and statuses; ready waits without installing; "Restart now" installs once; cancel reports `Cancelled` and installs nothing; a download error and a signature error each report their sentence with Retry.
- [ ] 4.3 Make the update item's primary action follow the state: "Install update" (idle), "Show progress" (running), and "Restart now" with the title "Dango 0.5.0 is ready to install" (ready). "Install later" pops the view and keeps the update ready. Verify with tests in `src/extensions/updates/mod.rs` for each item state and for "Install later".

## 5. Checks and review

- [ ] 5.1 Run `mise run check` and `cargo test`. Verify both pass.
- [ ] 5.2 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context. Report the findings to the author. Verify that the findings are listed in the session.

## Workflow follow-up

- Change 3 of the design-system v2 stack. Stack its branch on `progress-atoms` with git-spice.
- The install step itself (NSIS on Windows, bundle swap on macOS) does not change, only when it runs, so the change has no hand checks. A real update can be tried with the next release.
- Archive after review with `/opsx:archive`.
