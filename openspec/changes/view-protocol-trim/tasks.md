## 1. Remove command filtering

- [x] 1.1 Remove `Filtering` and the `filtering` fields of `ListView` and `GridView` in `src/protocol.rs`, drop the argument from `group` in `src/ui/state.rs` so it always filters, and drop the stale doc line on `view_sections`; verify `cargo clippy --all-targets -- -D warnings` passes
- [x] 1.2 Remove `filtering` from the clipboard, snippets, and system built-ins, the `src/invocation.rs` test helper, and their `Filtering::Launcher` asserts; verify `cargo test` passes
- [x] 1.3 Replace `a_command_filtered_list_shows_fifty` in `src/ui/tests.rs` with nothing (the other limit tests cover it) and drop `filtering` from the grid tree in `src/slint_shell/tests.rs`; verify `cargo test` passes, UI tests included

## 2. Refuse Shift or Alt shortcuts on other keys

- [x] 2.1 Move the version and column checks out of `Launcher::rendered` into one tree check in `src/ui/state.rs`, and call it from `rendered` and from the `Replaced` and `Removed` view outcomes in `action_finished`; verify with a new test in `src/ui/tests.rs` that an action returning a grid of two columns shows "This view can't be shown"
- [ ] 2.2 Add the shortcut rule to that check: a Shift or Alt shortcut whose key is not one ASCII letter makes the tree `ViewError::Invalid`, covering view, item, and empty state actions; verify with tests in `src/ui/tests.rs` that Shift+1 and Alt+/ are refused and that Ctrl+Shift+P runs its action
- [ ] 2.3 Document the letter rule on `Shortcut` in `src/protocol.rs` and reword the `PROTOCOL_VERSION` comment as the design says; verify `cargo clippy --all-targets -- -D warnings` passes

## 3. Review

- [ ] 3.1 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context; it reports findings only and the author decides what to fix
