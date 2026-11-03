# Tasks

## 1. Root accessories

- [ ] 1.1 Add `accessories: Vec<Accessory>` to `Candidate` (`src/search/mod.rs`), and `accessories` and `hotkey: Option<String>` to `ResultItem` (`src/ui/input.rs`). Fill them in `Core::result_item`: the hotkey from the config table's command settings for `Source::Command`, resolved for this platform. Give each permission warning a "Needs permission" danger tag. Verify with:
  - a test in `src/extensions/permissions/mod.rs` that the warning carries the tag;
  - an app test in `src/app/tests.rs` that a command with a configured hotkey gets it in its `ResultItem`.
- [ ] 1.2 Add `keys::hotkey_label(&str)` in `src/slint_shell/keys.rs`: "⌃⌥⇧⌘" order plus the key on macOS, "Ctrl+Alt+Shift+Win+V" on Windows, nothing for an unparsable string. Draw a root row's accessories and its hotkey chord in `result_row` (`src/slint_shell/render/list.rs`), at most three in all. Verify with:
  - unit tests for `hotkey_label` on each platform's `cfg`;
  - a UI test in `src/slint_shell/tests.rs` that a root row with a hotkey and a tag shows both.

## 2. AI runs as background tasks

- [ ] 2.1 Add `show: Option<String>` to `TaskState` (`src/tasks.rs`). In `src/ui/state.rs`, attach it to a success result toast as "Show answer" with `{ key: "Enter", modifiers: [Cmd] }`, and widen `shows_progress_of` to any view the owner pushed. Verify with reducer tests: a `Done` result with `show` gives a toast with that action; a result while the owner's detail view is on top gives no toast; the update's progress view still suppresses its toast.
- [ ] 2.2 In `src/extensions/ai/extension.rs`, run each request as a task, as design.md describes:
  - stop on the task's cancel signal instead of `ctx.is_cancelled()`;
  - keep the running task and its last text in `Inner`;
  - running the same command again re-shows the live run, and a different command cancels the old one first;
  - away from the view, an answer to show or paste is kept and ends `Done` with "<title> answer is ready" and `show`; an answer to copy is copied and ends `Done` with "Copied to clipboard";
  - add the `cancel` and `show` actions, and "Cancel" on the running view.

  Verify with tests in the module using its fake client:
  - a run keeps streaming after its context is superseded;
  - cancel stops it with no later views;
  - the same command again re-shows it;
  - a different command cancels it;
  - away-from-view endings for View, Paste (nothing inserted), and Copy (clipboard set);
  - `show` pushes the kept answer;
  - an in-view run still delivers as before.

## 3. Checks and review

- [ ] 3.1 Run `mise run check` and `cargo test`. Verify both pass.
- [ ] 3.2 macOS hand check with the real keyboard: a root row with a hotkey shows the macOS symbols, and Cmd+. cancels a background AI run from its busy toast. Leave open until a session on the Mac (with task-registry's 5.2).
- [ ] 3.3 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context. Report the findings to the author. Verify that the findings are listed in the session.

## Workflow follow-up

- Change 5 of the design-system v2 stack, stacked on `inline-answer` with git-spice.
- Archive after `task-registry` and `inline-answer`.
