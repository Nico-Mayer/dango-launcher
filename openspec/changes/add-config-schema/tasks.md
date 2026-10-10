## 1. Manifest descriptions and values

- [x] 1.1 Add `description: String` and `values: Vec<String>` to `PreferenceDecl` in `src-tauri/src/extension/manifest.rs`, make `Manifest::validate` reject an empty description with the message from design.md, and make the preference reader return the default for a string outside a non-empty `values`; verify with unit tests for the validation error and the out-of-set read, run `cargo test extension`
- [x] 1.2 Fill description and values on every built-in declaration: clipboard (five, `primary-action` with `paste` and `copy`) and quicklinks (`favicons`), using the sentences in design.md; verify every built-in manifest validates in the existing manifest tests and `cargo test extensions` passes
- [x] 1.3 Declare the AI extension's `provider` (string, no fixed values) and `thinking` (string, the four levels) in its manifest, which today lists no preferences and reads both keys from the config directly, and make its reads go through those declarations; verify the AI tests pass and a stored `thinking` of `deep` reads as the default

## 2. Config types

- [x] 2.1 In `src-tauri/src/config/mod.rs` add `schema: Option<String>` serialised as `$schema` as the first field of `Config`, make `FileConfig::ensure_file` set it to `./config.schema.json` on the file it creates, and update the `ensure_file` and Open Config File tests to expect `{ "$schema": "./config.schema.json", "version": 1 }`; verify a round-trip test keeps the key and `cargo test config` passes
- [x] 2.2 Drop `ProviderKind::Unknown`, move the `Thinking` and `Output` enums into `config` and use them on `CommandSettings`, and update the AI extension to consume the typed values; verify the AI extension tests pass and a config with `"kind": "mistral"` reports a parse error naming the provider
- [x] 2.3 In `src-tauri/src/config/hotkey.rs` expose `MODIFIER_TOKENS` and `KEY_TOKENS` as public constants and route `modifier` and `key_code` through them; verify every existing hotkey test still passes and a new test proves each token in the tables parses

## 3. Schema generator

- [x] 3.1 Add `schemars = "1"` to `src-tauri/Cargo.toml`, derive `JsonSchema` on every config struct and enum, and create `src-tauri/src/config/schema.rs` with `generate(manifests: &[Manifest]) -> serde_json::Value` that merges the derived schema with the per-extension objects from design.md; verify a test validates the generated schema against the JSON Schema meta-schema and `cargo build` succeeds
- [x] 3.2 Add the closing post-pass and the three reopened maps; verify with `jsonschema` tests that `hotkeys` under `launcher` is rejected, a user-named app, provider, and AI command are accepted, `prompt` under a window-management command is rejected, and an unknown provider `kind`, `thinking`, `output`, and `primary-action` are each rejected
- [x] 3.3 Add the hotkey `pattern` and the `hyperkey.tap` enum built from the token tables; verify a test that every modifier and key token, every lowercase letter and digit, and `f1` to `f12` match the pattern and that `hyper+lft` and `space` alone do not
- [x] 3.4 Add tests that every built-in preference key appears under its extension with type, default, description, and values, and that every shipped command id appears under `commands`, including the nine AI commands; verify `cargo test config::schema` passes

## 4. Written beside the config

- [x] 4.1 Add `config::schema::write(dir, manifests)` that compares before writing and logs the design.md line on failure, and call it from `setup` in `src-tauri/src/lib.rs` after the extensions are registered; verify with tests that a missing file is created, a differing file is replaced, an identical file keeps its modification time, and the own-write signal is untouched
- [x] 4.2 Run `tauri dev`; verify `config.schema.json` appears in the config directory, the tray status still reads `Config file loaded`, and `dango.log` has no reload line from the write

## 5. Settings reference

- [x] 5.1 Add the Markdown renderer and the `the_settings_reference_is_current` test with the `DANGO_UPDATE_DOCS` behaviour and failure text from design.md; verify the test fails on a missing `docs/config.md`, passes after `env DANGO_UPDATE_DOCS=1 cargo test the_settings_reference_is_current`, and fails again after editing one description without regenerating
- [x] 5.2 Generate `docs/config.md`, delete `docs/config.schema.json`, `docs/config.example.jsonc`, and the two tests that read them, and replace the README's configuration paragraph with the text from design.md, including a line that the schema file belongs in the dotfiles `.gitignore`; verify `rg config.example.jsonc` finds nothing outside `openspec/changes/archive` and `cargo test` passes

## 6. Windows verification

- [x] 6.1 Add `"$schema": "./config.schema.json"` to the live `config.json`, open it in the editor you use on Windows; verify completion offers `launcher`, `extensions`, and under `dango.clipboard.preferences` the five keys with their descriptions, and that `hotkeys` and `"thinking": "deep"` are flagged
- [x] 6.2 Type a hotkey with a misspelt key such as `hyper+lft`; verify it is flagged and that `hyper+left` is not
- [x] 6.3 Delete `config.schema.json`, start Dango; verify it is recreated and the config status is unchanged

## 7. macOS verification

Deferred: the Windows checks passed on 2026-10-10; these run the next time the macOS machine is at hand.

- [ ] 7.1 Add `"$schema": "./config.schema.json"` to the live `config.json`, open it in the editor you use on macOS; verify completion offers `launcher`, `extensions`, and under `dango.clipboard.preferences` the five keys with their descriptions, and that `hotkeys` and `"thinking": "deep"` are flagged
- [ ] 7.2 Delete `config.schema.json`, start Dango; verify it is recreated and the config status is unchanged
- [ ] 7.3 Make the config directory read-only, start Dango; verify it starts, the launcher works, and `dango.log` has the schema write line; restore permissions

## 8. Wrap up

- [x] 8.1 Review the diff for comments that restate the code and for scope beyond the proposal; verify `cargo clippy --all-targets -- -D warnings`, `cargo test`, and `npx svelte-check` pass
- [x] 8.2 Add one line to the M7 section of `openspec/ROADMAP.md` recording the generated schema and settings reference; verify it names `docs/config.md`
