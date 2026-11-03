# Tasks

## 1. One list of user files, applied to `config.toml`

- [x] 1.1 Add the user file list in `src/config/` with path, schema path, and header per file (only `config` for now), and make `config_path` read from it and `SCHEMA_REF` go; verify the existing config tests pass
- [x] 1.2 Add the shared write helper (temp file, own write, rename) and use it in `ensure_file`; change the created header to `#:schema ./.schemas/config.schema.json`; verify the `src/config/store.rs` header test and the system-commands "No config file exists yet" test
- [x] 1.3 Make `src/config/schema.rs` loop over the list, write each schema to `.schemas/<name>.schema.json` only when different, and stop writing `config.schema.json`; verify with tests for first write, replace, identical copy untouched, and refused write (config-schema "The schemas are written to the schemas folder")
- [x] 1.4 Log a bad config as `config.toml: <message>` in place of `config reload failed: the configuration file is not valid: ...`; verify with a test on the logged line, and that the existing `invalid_toml_is_reported_with_its_line` test passes
- [x] 1.5 Give `docs/config.md` a section per user file and regenerate it with `env DANGO_UPDATE_DOCS=1 cargo test the_settings_reference_is_current`; verify the reference test passes

## 2. `auth.toml`

- [x] 2.1 Add `auth` to the list, read it with the `toml` crate in `src/extensions/ai/auth.rs` with `version` reserved, and change the "couldn't be read" message to name `auth.toml`; verify the auth tests rewritten for TOML and the ai-providers "The key file cannot be read" test
- [x] 2.2 Build the auth parse error from the error span only, as `auth.toml: line <n> isn't valid TOML`; verify with a test that a broken `anthropic = "sk-ant-secret` gives the line and no `sk-ant-secret` (ai-providers "A broken key file shows no key")
- [x] 2.3 Add the auth schema (object of string values, `version` allowed) and its reference section; replace `docs/auth.example.json` with `docs/auth.example.toml` and update README; verify the schema test that a numeric key is rejected and the reference test

## 3. `snippets.toml` and `quicklinks.toml`

- [x] 3.1 Add `toml_edit` (the version already in `Cargo.lock`) and `slug` to `Cargo.toml`, then check the `slug` API in the vendored source; verify `cargo build`
- [x] 3.2 Add both files to the list, and rework `src/extensions/snippets/store.rs` to parse with `toml_edit::Document`, project tables to `Record` with type checks, treat `version` as reserved, and keep a `DocumentMut` for edits; update the record error strings and startup notice per design.md; verify the store tests rewritten for TOML pass, plus a test that `name = 3` is refused with its line and the last good records stay
- [x] 3.3 Make create, update, and remove edit only the touched table and write through the shared helper, adding the header when Dango creates the file; verify with tests that a comment, entry order, and an unknown key survive a create and an edit, a multi-line template is written on several lines, and a created file's first line is `#:schema ./.schemas/snippets.schema.json`
- [x] 3.4 Make ids with `slug` (empty slug becomes `snippet` or `quicklink`, `version` skipped, `-2` suffix when taken, rename keeps id) and remove the `name:<name>` fallback; verify with tests for "Café menu", a taken `github`, "!!!", and a rename
- [x] 3.5 Add `SnippetEntry` and `QuicklinkEntry` schema structs (doc comments as descriptions, `browser` enum from `Browser`, top level open to ids) and their reference sections; verify schema tests for a misspelt `templte` and an arbitrary id, and the reference test

## 4. Remove what is left of the JSON files

- [x] 4.1 Remove the `config.json` read from `examples/macos_keys_walkthrough.rs` (read `config.toml` or drop the part); verify `cargo clippy --all-targets -- -D warnings` builds the example
- [x] 4.2 Replace JSON file names in doc comments (`src/browser.rs`, `src/config/tables.rs`, `src/config/mod.rs`) and tests (`src/app/tests.rs`), then regenerate `docs/config.md`; verify `rg -n '(snippets|quicklinks|auth|config)\.json' src docs examples README.md AGENTS.md` finds nothing
- [x] 4.3 Update AGENTS.md: `auth.toml` in Architecture, and a trap that the `toml` error text quotes the source line, so a secrets file reports by span; verify by reading the diff

## 5. Integration

- [x] 5.1 Run `mise run check` and `cargo test`; verify both pass
- [ ] 5.2 Convert the author's `snippets.json`, `quicklinks.json`, and `auth.json` to TOML by hand (keep readable ids, slug the UUID ones), change the first line of `config.toml` to `#:schema ./.schemas/config.schema.json`, and delete `config.schema.json` and the old JSON files; nothing committed; verify a debug build lists the same snippets and quicklinks through the `dango-ui` `snapshot` tool, and an AI command with a hosted provider runs

## 6. Review

- [x] 6.1 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context; verify its findings are reported to the author, who decides what to fix

## Workflow follow-up

- Archive the change with `/opsx:archive` once every task is done.
