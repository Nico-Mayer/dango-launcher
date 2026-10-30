## 1. Config setting

- [x] 1.1 Make `PerPlatform` generic over its value with `String` as the default,
  keeping hotkeys and app names unchanged. Verify: existing `cargo test` in
  `src-tauri` passes.
- [x] 1.2 Add `path` to the extension entry as a list or a per-platform list,
  with a method returning this platform's directories with `~` expanded. Verify:
  `cargo test` unit tests for the plain list, `~` expansion, and an absent key.
- [ ] 1.3 Test on macOS that the per-platform form yields only the `macos` list.
  Verify: `cargo test` (cfg `target_os = "macos"`).
- [x] 1.4 Test on Windows that the per-platform form yields only the `windows`
  list. Verify: `cargo test` in Windows CI (cfg `windows`).
- [x] 1.5 Name the generic `PerPlatform` instance in the schema generator,
  allow `path` under `dango.ai` only, and regenerate `docs/config.md`. Verify:
  `cargo test the_settings_reference_is_current` and the schema tests.

## 2. Program lookup

- [x] 2.1 Add the `which` crate and confirm the `which_in` signature and its
  PATHEXT handling against the vendored source under `~/.cargo/registry`.
- [x] 2.2 Carry the directories from `Providers::from_config` into the request
  for a `cli` provider.
- [x] 2.3 In `CliClient`, resolve a bare command with `which_in` over the
  directories then Dango's PATH, spawn the full path, and set the child's
  `PATH` to the same list. Keep `CommandMissing` when nothing is found. Verify:
  `cargo test` with a script in a temp directory that is not on PATH, found by
  its bare name.
- [x] 2.4 Test that the first listed directory wins and that a command given as
  a path skips the lookup. Verify: `cargo test`.
- [x] 2.5 Test that a program started by the provider's program finds a tool
  only in a configured directory. Verify: `cargo test`.
- [x] 2.6 Test on Windows that a bare name finds a `.cmd` file in a configured
  directory. Verify: `cargo test` in Windows CI (cfg `windows`).
- [x] 2.7 Test that a program in no directory and not on PATH still gives the
  "couldn't run" message. Verify: existing
  `a_command_that_is_not_installed_names_itself`.

## 3. Real-world check

- [ ] 3.1 Windows: with `~/AppData/Local/mise/shims` in the `windows` list,
  start Dango from autostart (not a terminal) and run an AI command on the
  `claude` provider. It answers. A tray launch's inherited PATH cannot be
  reproduced in a test.
