## 1. Manifest

- [x] 1.1 Remove `command_id` and `required` from `PreferenceDecl`, and the `effective` and `is_unsatisfied` methods with their tests, in `src/extension/manifest.rs`; update the declarations in `src/extensions/clipboard/extension.rs`, `src/extensions/snippets/extension.rs`, and `src/extensions/ai/extension.rs`. Verify: `cargo test extension::manifest`.

## 2. Store and config

- [x] 2.1 Drop the `command_id` parameter from `PreferenceStore::get`/`set` and from `Preferences::read`/`declaration` in `src/extension/preferences.rs`; remove the `extension_and_command_scopes_with_one_key_stay_apart` test and update the test stores there and in `src/extensions/clipboard/extension.rs`. Verify: `cargo test extension::preferences` and `cargo test clipboard`.
- [x] 2.2 Drop the command argument from `Config::preference` and the `preferences` field from `CommandSettings` in `src/config/mod.rs`; drop the command branch in `FileConfig::get`/`set` in `src/config/store.rs`, keeping the write and persist; update the callers in `src/extensions/ai/commands.rs`. Verify: `cargo test config` and `cargo test ai`.
- [x] 2.3 Remove the command-scope filter in `preferences()` in `src/config/schema.rs` and regenerate `docs/config.md` with `env DANGO_UPDATE_DOCS=1 cargo test the_settings_reference_is_current`. Verify: `cargo test schema` passes and the command `preferences` rows are gone from `docs/config.md`.

## 3. Wrap up

- [ ] 3.1 Run `hk check` and `cargo test`. Verify: both pass.
- [ ] 3.2 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context. The review only reports findings; the author decides what to fix.
