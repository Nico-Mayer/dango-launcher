# Tasks

## 1. Hotkey chords on root rows

- [ ] 1.1 Add `hotkey: Option<HotKey>` to `ResultItem` (`src/ui/input.rs`), filled in `Core::result_item` from `services.hotkeys`' registered bindings (commands and applications). Verify with an app test in `src/app/tests.rs`: a command with a registered hotkey gets it, one whose chord was refused gets none.
- [ ] 1.2 Add `keys::hotkey_label(&HotKey)` in `src/slint_shell/keys.rs` for macOS ("⌃⌥⇧⌘" order, then the key). Verify with a `#[cfg(target_os = "macos")]` unit test ("ctrl+alt+v" → "⌃⌥V"). It runs on the Mac.
- [ ] 1.3 The same for Windows ("Ctrl+Alt+Shift+Win+" then the key). Verify with a `#[cfg(windows)]` unit test ("ctrl+alt+v" → "Ctrl+Alt+V").
- [ ] 1.4 Draw a root row's hotkey as a key chord accessory in `result_row` (`src/slint_shell/render/list.rs`). Verify with a UI test in `src/slint_shell/tests.rs`: a root row with a hotkey shows its label, and one without shows none.

## 2. AI runs as background tasks

- [ ] 2.1 Add `show: Option<String>` to `TaskState` (`src/tasks.rs`). In `src/ui/state.rs`, attach it to a success result toast as "Show answer" with `{ key: "Enter", modifiers: [Cmd] }`. Verify with reducer tests: a `Done` result with `show` gives a toast with that action, and its chord runs the owner's `show` action.
- [ ] 2.2 Give the AI extension a `visible` probe from Core (`src/app/startup.rs`, `src/app/mod.rs`). Verify with `cargo test`.
- [ ] 2.3 In `src/extensions/ai/extension.rs`, run each request as a task, as design.md describes:
  - start the task after `prepare`, and cancel another command's task then;
  - keep `Inner.live` and the last streamed text, and stop on the task's cancel signal;
  - running the same command again re-shows it;
  - in view: deliver as today, ending `Cancelled`;
  - away (context cancelled or launcher hidden): show and paste keep the answer and end `Done` with "Answer ready: <title>" and `show`; copy copies and ends `Done` with "Copied to clipboard"; a failure ends `Failed`;
  - the actions `background`, `cancel` (⌘.), and `show`.

  Verify with tests in the module using its fake client:
  - a run keeps streaming after its context is cancelled;
  - cancel stops it with no later views;
  - the same command again re-shows it;
  - another command cancels it only once its prompt is ready, so a form or a missing selection doesn't;
  - away endings for show, paste (nothing inserted), and copy (clipboard set), including while hidden but held;
  - an in-view paste still inserts and posts no task result;
  - `show` pushes the kept answer;
  - Enter on the running view is "Continue in background".

## 3. Checks and review

- [ ] 3.1 Run `mise run check` and `cargo test`. Verify both pass.
- [ ] 3.2 Windows hand check with the real keyboard (`mise run keys`, `mise run shot`):
  - press an AI paste command's hotkey in a text field and wait: the answer is pasted there;
  - press it again, then click into another app before the answer arrives: nothing is pasted, and the next open shows "Answer ready" with "Show answer".

  Verify with screenshots.
- [ ] 3.3 macOS hand check with the real keyboard: Cmd+. cancels a running AI request from its busy toast (winit delivers it through `cancelOperation:`), and the paste-command hotkey flow from 3.2 behaves the same. Leave open until a session on the Mac, with task-registry's 5.2.
- [ ] 3.4 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context. Report the findings to the author. Verify that the findings are listed in the session.

## Workflow follow-up

- Change 5 of the design-system v2 stack, stacked on `inline-answer` with git-spice.
- Archive last: its `background-tasks` delta needs `task-registry` archived first.
