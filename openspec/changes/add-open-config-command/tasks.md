## 1. Config file creation

- [x] 1.1 Add `FileConfig::ensure_file` in `src-tauri/src/config/store.rs` that returns the path, creating the file with the in-memory config and `version` 1 when absent, through the same directory-create, own-write, and write steps `persist` uses; verify with unit tests in a temp directory that a missing file is created with exactly `{ "version": 1 }` and the text lands in `own_write`, that an existing file, including an unparseable one, is returned byte-identical, and that a read-only directory yields an `Err` naming the path; run `cargo test config` from `src-tauri`

## 2. The command

- [x] 2.1 Add a `system::OpenFile` trait with `open(&self, path: &Path) -> Result<(), String>`, the `COMMAND_OPEN_CONFIG` id `open-config`, the manifest entry titled `Open Config File` with icon `settings`, `NoView` mode, and the keywords from design.md, and the two new constructor arguments on `SystemExtension::new`; verify the existing manifest tests are updated to five commands and `cargo test system` passes once a fake opener is added to the test module
- [x] 2.2 Implement the `OpenConfigFile` command in `src-tauri/src/extensions/system/commands.rs`: call `ensure_file`, then the opener, mapping each failure to its banner string from design.md and writing the `config: could not create` or `config: could not open` log line with the raw error; verify with tests that a missing file is created and the fake opener receives its path, that an existing file is opened untouched, that an opener error finishes with `Outcome::Failure` carrying the open string, and that a create error finishes with the create string and the opener is never called
- [x] 2.3 In `src-tauri/src/lib.rs`, rename `DefaultBrowser` to `DefaultHandler`, implement `system::OpenFile` on it with `opener().open_path(path, None::<&str>)`, and pass the handler and `file_config.clone()` to `SystemExtension::new`; verify `cargo clippy --all-targets -- -D warnings` and `cargo test` pass and `tauri dev` shows the command in root search for "settings", "keybinds", and "config"

## 3. Icon and documentation

- [x] 3.1 Add the Lucide `settings` icon to the registry in `src/lib/Icon.svelte` as `settings`, imported individually; verify `npx svelte-check` passes and the command row in `tauri dev` shows a gear rather than a placeholder
- [x] 3.2 Add the two README sentences from design.md to the configuration section; verify the command title in the text matches the manifest exactly

## 4. Windows verification

- [ ] 4.1 With `config.json` present, run Open Config File from root search; verify the file opens in the application Windows associates with `.json`, the launcher hides, the editor is in the foreground rather than behind the previous app, and the file content is unchanged
- [ ] 4.2 Move `config.json` aside and run the command; verify a file containing only `"version": 1` is created in the config directory and opened, and the tray status still reads `Config file loaded` with no reload line in `dango.log`
- [ ] 4.3 Remove the `.json` association in Settings > Apps > Default apps, run the command; verify the Windows "How do you want to open this file?" chooser appears and the launcher has hidden, then restore the association
- [ ] 4.4 Set `$DANGO_CONFIG_DIR` to an empty directory, start Dango, run the command; verify the file is created and opened there, not under `~/.config/dango`
- [ ] 4.5 Make the config directory read-only with no file present, run the command; verify the banner reads `Couldn't create the config file. See dango.log for the reason.`, the log names the path, and no editor opens; restore permissions
- [ ] 4.6 Bind the command under `extensions.dango.system.commands.open-config.hotkey`, press the chord from another application; verify the file opens without the launcher appearing

## 5. macOS verification

- [ ] 5.1 With `config.json` present, run Open Config File from root search; verify the file opens in the application macOS associates with `.json`, the launcher hides, and the file content is unchanged
- [ ] 5.2 Move `config.json` aside and run the command; verify a file containing only `"version": 1` is created in the config directory and opened, and the tray status still reads `Config file loaded` with no reload line in `dango.log`
- [ ] 5.3 Set `$DANGO_CONFIG_DIR` to an empty directory, start Dango, run the command; verify the file is created and opened there, not under `~/.config/dango`
- [ ] 5.4 Make the config directory read-only with no file present, run the command; verify the banner reads `Couldn't create the config file. See dango.log for the reason.`, the log names the path, and no editor opens; restore permissions
- [ ] 5.5 Bind the command under `extensions.dango.system.commands.open-config.hotkey`, press the chord from another application; verify the file opens without the launcher appearing

## 6. Wrap up

- [x] 6.1 Review the diff for comments that restate the code, for any frontend or capability change beyond the icon, and for scope beyond the proposal; delete what is found and verify `cargo clippy --all-targets -- -D warnings`, `cargo test`, and `npx svelte-check` pass
