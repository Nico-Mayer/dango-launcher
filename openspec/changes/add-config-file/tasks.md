## 1. The config schema and loader

Pure Rust, no platform and no app wiring, so it is finished and tested first.

- [x] 1.1 Add a `config` module with the typed `Config` shape (version, launcher hotkey, and per-extension enabled/preferences/commands), each level keeping a catch-all for unknown keys, and parse it from JSON with a test that a full file round-trips and a test that an unknown key is preserved through parse and re-serialise
- [x] 1.2 Resolve missing sections to built-in defaults: no file, an empty file, and a file omitting a section all yield a usable config, with tests
- [x] 1.3 Reject a malformed file with an error that names the problem rather than panicking, with a test over invalid JSON and over a wrong-typed value
- [x] 1.4 Serialise a `Config` back to JSON preserving every unknown key and unchanged setting, with a test that a load-modify-write cycle keeps unrelated and unrecognised keys

## 2. The hotkey grammar

- [x] 2.1 Parse a hotkey string of portable modifier tokens (`mod`, `ctrl`, `alt`, `shift`, `meta`, `hyper`) plus one physical key into modifiers and a `Code`, resolving `mod` to Ctrl on Windows and Cmd on macOS, with tests over single and multiple modifiers and over an unknown token
- [x] 2.2 Accept a per-platform hotkey object (`{ "macos": ..., "windows": ... }`) and pick this platform's value, with a test
- [x] 2.3 Reserve but do not bind `hyper`: parsing a `hyper+...` chord succeeds and is representable, so a later change can bind it, with a test that it parses

## 3. The config directory and file-backed stores

- [x] 3.1 Resolve the config directory as `~/.config/dango/` on both platforms with a `$DANGO_CONFIG_DIR` override, creating it if absent, with tests for the default and the override
- [x] 3.2 Implement a `FileConfig` that holds the loaded config behind a shared lock and implements `PreferenceStore` and `EnabledStore`, returning file values where present and letting the reader fall back to the declared default otherwise, with tests against the existing preference and enabled readers
- [x] 3.3 Serialise a typed JSON preference value to the string form the `PreferenceStore` reader expects, with tests over a number, a boolean, and a string
- [x] 3.4 Add a forward-only migration dropping the `preferences` and `extension_state` tables, and confirm it applies to an existing database without touching the other tables
- [x] 3.5 Wire `lib.rs` to load the config at startup and hand `FileConfig` to the extension host in place of the SQLite-backed stores, and verify the existing suite still passes

## 4. The launcher hotkey and aliases from the file

- [x] 4.1 Register the launcher hotkey from the config, falling back to the platform default when unset, replacing the hardcoded Alt+Space, and verify the app still summons on the default with no file
- [x] 4.2 Apply a configured command alias over the manifest's when building command candidates, with a test that a file alias reaches search and a command with no file alias keeps its manifest one

## 5. Live reload and surfacing errors

- [x] 5.1 Watch the config file with `notify`, debounced, and on a clean parse swap the config, re-register the launcher hotkey if it changed, and enable or disable extensions whose state changed through `ExtensionHost::set_enabled`, with tests over the apply logic driven by hand
- [x] 5.2 Suppress the app's own write so a file Dango wrote is not re-applied as an external edit, reusing the own-write pattern, with a test
- [x] 5.3 Keep the last good config and surface the error through a tray item and a log file when a reload fails to parse, with a test over the last-good fallback
- [x] 5.4 On startup with an unparseable file, run on defaults and surface the error rather than refusing to start, with a test

## 6. Documentation and example

- [x] 6.1 Add a `$schema` JSON Schema for the config and a commented example config in the repo (documentation, not the live file), and confirm the example validates against the schema

## 7. Verification

- [ ] 7.1 Confirm on both platforms that a `config.json` at `~/.config/dango/` applies: a changed launcher hotkey summons and the default no longer does, a disabled extension is gone from search, and a preference override takes effect
- [ ] 7.2 Confirm on both platforms that editing the file while running re-applies it without a restart
- [ ] 7.3 Confirm on both platforms that a malformed file keeps the last good config, shows the tray error, and writes the log, and that a malformed file at startup falls back to defaults
- [ ] 7.4 Confirm on both platforms that `$DANGO_CONFIG_DIR` relocates the directory and that removing the file returns everything to defaults
