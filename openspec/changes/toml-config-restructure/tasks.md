# Tasks

## 1. Read `config.toml`

- [x] 1.1 Add `toml` as a direct dependency and parse `Config` with `toml::from_str`; keep the empty-file case; verify `cargo test config` with every config fixture in `src/config/` rewritten as TOML, and a test that a syntax error's message names its line
- [x] 1.2 Rename the file to `config.toml` in `location.rs`, the watcher, and Open Config File; verify the watcher and system command tests (`cargo test watch`, `cargo test system`) with fixtures named `config.toml`, and a test that a `config.json` alone in the directory loads as defaults
- [ ] 1.3 Drop the `$schema` field from `Config` and make the created file start with `#:schema ./config.schema.json`; verify a `store.rs` test that `ensure_file` writes that first line and the result parses
- [x] 1.4 Rewrite every JSON config literal in tests outside `src/config/` (`app/tests.rs`, `hotkeys.rs`, `ai/commands.rs`, others found by `rg 'r#"\{' src`) as TOML in the current structure; verify `cargo test` passes
- [ ] 1.5 Update README's configuration section for `config.toml` and the `#:schema` line, regenerate `docs/config.md` (`env DANGO_UPDATE_DOCS=1 cargo test the_settings_reference_is_current`) with its header naming `config.toml`; verify `the_settings_reference_is_current` passes

## 2. Typed tables per built-in

- [ ] 2.1 Add the `ExtensionTable` trait, `CommandBinding`, the shared `Table`, the id-to-table-name function, and `Config::table(id)` with the `extensions` fallback; verify unit tests that `table("dango.clipboard")` reads `[clipboard]`, an unknown id reads `extensions."author.x"`, and `enabled`/`command_enabled`/`alias` answer through it
- [ ] 2.2 Give `Config` one field per built-in using `Table`, remove `extensions: BTreeMap<String, Extension>` as the built-in store, and route Core (`app/mod.rs` alias lookup, `store.rs` enabled state) and `hotkeys.rs` through `Config::table`; verify `cargo test` including the hotkey and enabled-state tests, with fixtures moved to top-level tables
- [ ] 2.3 Add `ClipboardTable` with typed `entries`, `entry-megabytes`, `total-megabytes`, `excluded-applications: Vec<String>`, and a `PrimaryAction` enum, defaults in `Default`; hand the clipboard extension the shared config instead of a `Preferences` reader and delete the comma splitting; verify clipboard tests for the defaults, a set value, case-insensitive exclusion from a list, and that `primary-action = "drop"` fails to parse naming the key
- [ ] 2.4 Add the quicklinks table with `favicons: bool` (default true) and read it from the shared config in `startup.rs`; verify the favicon test with `favicons = false`
- [ ] 2.5 Add `AiTable` with `providers`, `provider`, `thinking: Thinking`, and `commands: BTreeMap<String, AiCommand>`; make `output` and `thinking` serde enums and remove `Thinking::parse`, `Thinking::NAMES`, `Output::parse`; strip the AI fields from `CommandSettings`; switch `ProviderSettings` to kebab-case; verify `cargo test ai` including a user-defined command with a multi-line `"""` prompt, `base-url` read, and `thinking = "max"` failing to parse
- [ ] 2.6 Rename `apps` to `hotkeys` in the applications table; verify the application hotkey tests with `[applications.hotkeys.<name>]`
- [ ] 2.7 Remove `Preferences`, `PreferenceStore`, `PreferenceDecl`, `PreferenceKind`, `Manifest::preferences`, and the preference count in the "contributes nothing" check; verify `cargo test` and `cargo clippy --all-targets -- -D warnings` pass with no dead code left
- [ ] 2.8 Rebuild `schema.rs` on the new structs: drop the preference section builder and `AI_ONLY`, close each built-in table's `commands` to its shipped ids, leave `ai.commands`, `ai.providers`, `applications.hotkeys`, and `extensions` open, close the top level; verify the schema tests for a misspelt key, an unknown top-level table, `prompt` outside `[ai]`, enumerated values, and a user-named entry, all against TOML fixtures converted to JSON values
- [ ] 2.9 Regenerate `docs/config.md`; verify `the_settings_reference_is_current` passes and the reference lists `[clipboard]` settings with types and defaults and no `preferences` row

## 3. Layout rules

- [ ] 3.1 Give `Hyperkey` an `enabled` field defaulting to false and replace `Option<Hyperkey>` checks with it; verify hyperkey config tests: no table is off, a table without `enabled` is off, `enabled = true` alone is on with Shift and no tap key
- [ ] 3.2 Make `selection.excluded-applications` a `Vec<String>`; verify the selection refusal tests read a list and match ignoring case
- [ ] 3.3 Rename `launcher.reset-after` to `reset-after-seconds`; verify the grace period tests and that the schema rejects `-5` and `1.5`
- [ ] 3.4 Add a test over all built-in manifests that no table name is `version`, `launcher`, `hyperkey`, `selection`, or `extensions`, and that no table struct has a setting named `enabled` or `commands` other than the reserved fields; verify it passes and fails when a manifest id is changed to `dango.launcher`
- [ ] 3.5 Regenerate `docs/config.md` and run `mise run check`; verify both pass

## 4. Review

- [ ] 4.1 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context; record its findings here for the author to decide on

## Workflow follow-up

- Archive `add-agent-cli-providers` before applying this change.
- Before installing the release with this change, rewrite the author's
  `~/.config/dango/config.json` as `config.toml`, including `enabled = true`
  in `[hyperkey]`, or autostart turns off.
- Open the new `config.toml` in the author's editor once; if completion or
  checking is missing, switch schemars to draft-07 (see design, risks).
