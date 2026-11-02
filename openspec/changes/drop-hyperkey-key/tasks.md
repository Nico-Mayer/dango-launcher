# Tasks

## 1. Drop the key from config, schema, and start path

The field is read in all of these, so they change together to keep the build
green.

- [ ] 1.1 Remove `Hyperkey::key` and `default_hyperkey_key` from `src/config/mod.rs`; update the hyperkey parse tests to blocks without `key`, and add tests that `{ "hyperkey": {} }` parses with Shift on and no tap, and that a leftover `"key": "f13"` still parses. Verify with `cargo test config::`.
- [ ] 1.2 Remove `HYPERKEY_KEYS` and the `key` enum from `constrain_hyperkey` in `src/config/schema.rs`; drop `key` from the `FULL` sample; replace the `"key": "tab"` rejection with a test that the schema rejects any `key` in the hyperkey block. Verify with `cargo test config::schema`.
- [ ] 1.3 In `src/app/mod.rs`, build the hyperkey spec with `HyperkeyTrigger::CapsLock` and compare only `(shift, tap)` for live reload; remove the "not a remappable key" path, and remove `HyperkeyTrigger::from_key` and its test from `src/platform/mod.rs`. Leave `src/platform/macos/` untouched. Verify with `cargo test` (all green).
- [ ] 1.4 Regenerate `docs/config.md` with `env DANGO_UPDATE_DOCS=1 cargo test the_settings_reference_is_current` and confirm the only diff is the removed `key` row; verify `cargo test the_settings_reference_is_current` passes without the variable.
- [ ] 1.5 Run `mise run check` (fmt, clippy with `-D warnings`) and verify it passes.

## 2. Review

- [ ] 2.1 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context. The review reports findings only; the author decides what to fix.

## Workflow follow-up

- Reference `Closes #24` in the commit that removes the field.
- Archive the change with `/opsx:archive` once the review is handled.
