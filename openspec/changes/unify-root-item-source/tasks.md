# Tasks

## 1. Root rows name their extension

The candidate field rename touches every provider, so this group lands as one
build-green step.

- [x] 1.1 Rename `Candidate::subtitle` to `status` in `src/search/mod.rs` and update every construction site (`src/extension/mod.rs`, `src/extension/registry.rs`, `src/ranking/mod.rs`, test fixtures). Only `permissions` and `updates` set it. `applications`, `games`, and `snippets` (both kinds) set nothing. Verify with `cargo build --all-targets`.
- [x] 1.2 In `Core::result_item` (`src/app/mod.rs`), set `ResultItem::subtitle` to `c.status` when `c.pinned`, else to `context.title`. Add tests in `src/app/tests.rs` that a non-pinned root item shows its extension's name, a command shows its extension's name, and a pinned item shows its status line. Verify with `cargo test app::`.
- [x] 1.3 Update the extension tests that asserted the old texts: `applications` (`/a/code`), `games` (`Steam`), `snippets` (body preview in root), `permissions` and `updates` (now on `status`). Verify with `cargo test extensions::`.
- [x] 1.4 Run `cargo test` (UI tests included) and `mise run check`, and verify both pass.

## 2. Remove the command subtitle from the manifest

- [ ] 2.1 Remove `CommandDecl::subtitle` from `src/extension/manifest.rs` and from every built-in manifest (`clipboard`, `system`, `window_management`, `games`, `ai`, `updates`, `snippets`) and test fixture. `command_candidates` no longer reads it. Verify with `cargo test` and `mise run check`.

## 3. Review

- [ ] 3.1 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context. The review reports findings only; the author decides what to fix.

## Workflow follow-up

- Archive the change with `/opsx:archive` once the review is handled.
