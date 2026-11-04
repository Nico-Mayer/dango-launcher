# Tasks

## 1. One chord format

- [x] 1.1 On Windows: `Label::ChordSeparator` becomes " ", and `panel_label` and `confirm_label` (`src/slint_shell/keys.rs`) join with it. Verify with `#[cfg(windows)]` unit tests: the panel chord reads "Ctrl K", the confirm chord "Ctrl ↵", a `{ key: "Enter", modifiers: [Cmd] }` shortcut "Ctrl ↵", and ctrl+alt+v "Ctrl Alt V". Update the existing Windows hotkey test.
- [ ] 1.2 On macOS: the same joins give "⌘K", "⌘↵", and "⌃⌥V". Verify with `#[cfg(target_os = "macos")]` unit tests, and update `a_kbd_shows_the_platform_chord` in `src/slint_shell/tests.rs` from "⌘ K" to "⌘K". They run on the Mac.
- [x] 1.3 Make the toast's key chord in `ui/shell/action-bar.slint` keep its preferred width. Verify with a UI test in `src/slint_shell/tests.rs` that fails before the fix: the toast's chord chip is as wide as the footer's chip for the same text.

## 2. A toast with an action stays

- [x] 2.1 In `src/ui/state.rs`, `show_success` takes the toast's action and schedules no expiry when there is one; `show_task_result` passes "Show answer" in. `clear_failure` becomes `clear_lasting` and also clears a toast with an action. Verify with reducer tests in `src/ui/tests.rs`: a `Done` result with `show` schedules no `ExpireToast`; a key press clears it without running the action; a success toast without an action still expires.

## 3. The HUD and the jump

- [x] 3.1 In `task_changed`, a `Done` or newly arrived `NeedsInput` while hidden shows a success HUD with its sentence, carrying "Show answer" when the result has `show`. Verify with reducer tests: a hidden `Done` sets a success HUD and still toasts on the next open; a visible `Done` shows no HUD; a hidden `NeedsInput` reported twice shows the HUD once.
- [x] 3.2 `Event::Activated` gains `asked`, set only by `Core::toggle`. When `asked` and the HUD on screen carries an action, the reducer pushes `Effect::RunAction` with it and shows no toast. Verify with reducer tests: an asked open during the HUD runs the action once and shows no toast; an open after `ToastExpired` shows the toast; an open that is not asked (a command hotkey) does not jump; a newer copy HUD in between means no jump; a failure never jumps. Verify `toggle` sets `asked` and `dispatch_command` does not with app tests in `src/app/tests.rs`.
- [x] 3.3 In `Activated`, when `held` and the top view belongs to the result's owner, show no toast and do not jump (`shows_view_of`: same owner and same title). Verify with reducer tests: reopening held onto an AI detail from the result's owner shows no toast; the same while its HUD still shows does not jump; after a reset the toast shows with "Show answer"; the update's progress view keeps its rule.

## 4. Checks and review

- [x] 4.1 Run `mise run check` and `cargo test`. Verify both pass.
- [x] 4.2 Windows hand check through `dango-ui`, `mise run keys`, and `mise run shot`: run "Fix Spelling and Grammar" on a Notepad selection and switch to Notepad before it finishes.
  - The HUD reads "Answer ready: Fix Spelling and Grammar".
  - Opening the launcher with its hotkey while the HUD shows lands on the answer.
  - A second run, opened after the HUD is gone, shows the toast with "Show answer" and "Ctrl ↵", and the toast stays until a key press.

  Verify with screenshots.
- [ ] 4.3 macOS hand check of the same flow, with "⌘↵" on the toast. Leave open until a session on the Mac.
- [x] 4.4 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context. Report the findings to the author.

## Workflow follow-up

- Stacked on `adopt-components` with git-spice. Archive after it and after
  `task-registry`: the `root-search`, `ai-commands`, and `background-tasks`
  deltas build on `adopt-components`, and the `design-system` delta on
  `task-registry`.
