## 1. Remove the client

- [x] 1.1 Delete `extensions/ai/cli.rs`, the `CliClient` export, `CliSpec`, and
  the `cli` field on `Request`. Collapse `Clients` so the extension gets the
  `genai` client directly, and drop the wiring in `lib.rs`. Verify: `cargo test`
  in `src-tauri`.
- [x] 1.2 Remove `ProviderKind::Cli` and its branches in `commands.rs`,
  `client.rs`, and `provider.rs` (`is_local`). Add a test that a provider with
  `"kind": "cli"` is a config error naming the kind, like any unknown kind
  (`a_program_is_not_a_provider_kind`). Verify: `cargo test`.
- [x] 1.3 Remove `process-wrap` and `which` from `Cargo.toml`. Verify:
  `cargo clippy --all-targets -- -D warnings`.

## 2. Remove the config surface

- [x] 2.1 Remove the `command` and `args` provider fields, the `path`
  extension field, and `SearchPath`. Return `PerPlatform` to a plain string
  value. Verify: `cargo test` config tests.
- [x] 2.2 Remove `constrain_search_path`, the `SearchPath` title, and the
  `path` removal in `schema.rs`; drop the search path schema test. Regenerate
  `docs/config.md`. Verify:
  `env DANGO_UPDATE_DOCS=1 cargo test the_settings_reference_is_current`, then
  `cargo test` with no diff.

## 3. Docs and planning

- [x] 3.1 Remove the tray PATH and `.cmd` lookup trap from `AGENTS.md`.
- [x] 3.2 Delete `openspec/changes/add-ai-cli-path/`. Verify:
  `openspec list` no longer shows it.
- [x] 3.3 Run `hk check`. Verify: it passes.

## 4. Author's config

- [ ] 4.1 In `~/.config/dango/config.json`, set `dango.ai` `preferences.provider`
  to `local`, remove the `claude` provider, the `path` key, and every command's
  `"provider": "claude"`. Verify: the file parses with `jq`, and an AI command
  run from the release build answers through Ollama.
