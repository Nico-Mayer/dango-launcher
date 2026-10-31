## 1. One limit for root and views

- [x] 1.1 Move the limit of 50 into one constant in `src/ui` and make `src/app`'s `RESULT_LIMIT` use it; verify `cargo test` passes unchanged (root search tests in `src/app/tests.rs` and `src/search`)

## 2. Cap pushed lists and grids

- [x] 2.1 Make `group` in `src/ui/state.rs` stop after the limit, counting loose items first and then each section in order, after the query filter; verify with a `src/ui/tests.rs` test that a 500-item list shows the first 50 and that a query matching only the 300th item shows it
- [x] 2.2 Add a `src/ui/tests.rs` test for two sections of 40 items: all 40 of the first and the first 10 of the second are entries, and a section with no shown item is not drawn
- [x] 2.3 Add `src/ui/tests.rs` tests that a command-filtered list of 120 items and a grid of 80 tiles each show 50
- [x] 2.4 Add a `src/ui/tests.rs` test that Arrow Down on the 50th of 500 items keeps the 50th selected, and that the action panel opens for it
- [x] 2.5 Add a UI test in `src/slint_shell/tests.rs` that a pushed list of 500 items renders 50 rows; verify with `cargo test`

## 3. Docs

- [x] 3.1 Update the ListView note in AGENTS.md ("it holds at most 50") to say every list holds at most 50, root and pushed; verify by reading the note
- [x] 3.2 Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`; verify all pass
