# Tasks

## 1. One chord format

- [ ] 1.1 On Windows: `Label::ChordSeparator` becomes " ", and `panel_label` and `confirm_label` (`src/slint_shell/keys.rs`) join with it. Verify with `#[cfg(windows)]` unit tests: the panel chord reads "Ctrl K", the confirm chord "Ctrl ↵", a `{ key: "Enter", modifiers: [Cmd] }` shortcut "Ctrl ↵", and ctrl+alt+v "Ctrl Alt V". Update the existing Windows hotkey test.
- [ ] 1.2 On macOS: the same joins give "⌘K", "⌘↵", and "⌃⌥V". Verify with `#[cfg(target_os = "macos")]` unit tests. They run on the Mac.
- [ ] 1.3 Make the toast's key chord in `ui/shell/action-bar.slint` keep its preferred width. Verify with a UI test in `src/slint_shell/tests.rs`: a toast with "Show answer" draws its chord chip no wider than the footer's chip for the same text.

## 2. A toast with an action stays

- [ ] 2.1 In `src/ui/state.rs`, `show_success` takes the toast's action and schedules no expiry when there is one; `show_task_result` passes "Show answer" in. `clear_failure` becomes `clear_lasting` and also clears a toast with an action. Verify with reducer tests in `src/ui/tests.rs`: a `Done` result with `show` schedules no `ExpireToast`; a key press clears it without running the action; a success toast without an action still expires.

## 3. The HUD and the jump

- [ ] 3.1 In `task_changed`, a `Done` or newly arrived `NeedsInput` while hidden shows a success HUD with its sentence, and `pending_result` remembers that HUD's id. Verify with reducer tests: a hidden `Done` sets a success HUD and still toasts on the next open; a visible `Done` shows no HUD.
- [ ] 3.2 In `Event::Activated`, a pending result with `show` whose HUD still shows pushes `Effect::RunAction` with the "Show answer" request and shows no toast. Verify with reducer tests: opening during the HUD runs the action once and shows no toast; opening after `ToastExpired` for that HUD shows the toast; a newer HUD (a copy) in between means no jump; a failure never jumps.
- [ ] 3.3 In `src/extensions/ai/extension.rs`, a run that ends while its view is held keeps no answer and reports `Done` without `show`. Verify by updating `a_paste_command_finishing_while_hidden_pastes_nothing`: the held view shows the answer, the report has no `show`, and nothing is kept; `a_run_left_behind_keeps_its_answer_for_later` still passes.

## 4. Checks and review

- [ ] 4.1 Run `mise run check` and `cargo test`. Verify both pass.
- [ ] 4.2 Windows hand check through `dango-ui`, `mise run keys`, and `mise run shot`: run "Fix Spelling and Grammar" on a Notepad selection and switch to Notepad before it finishes.
  - The HUD reads "Answer ready: Fix Spelling and Grammar".
  - Opening the launcher while the HUD shows lands on the answer.
  - A second run, opened after the HUD is gone, shows the toast with "Show answer" and "Ctrl ↵", and the toast stays until a key press.

  Verify with screenshots.
- [ ] 4.3 macOS hand check of the same flow, with "⌘↵" on the toast. Leave open until a session on the Mac.
- [ ] 4.4 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context. Report the findings to the author.

## Workflow follow-up

- Stacked on `adopt-components` with git-spice. Archive after it: the
  `root-search`, `ai-commands`, and `background-tasks` deltas build on its
  wording.
