## 1. Config setting

- [x] 1.1 Add `autostart: Option<bool>` to `Launcher` in `src-tauri/src/config/mod.rs`, include it in `is_empty`, and extend the existing config round-trip tests so a file with `launcher.autostart` loads the value, an absent key reads `None`, and writing back preserves it; verify with `cargo test config` from `src-tauri`
- [x] 1.2 Document the setting in `docs/config.example.jsonc` inside `launcher` with the comment text from design.md; verify the file still parses as JSONC by stripping comments and feeding it to `jq`

## 2. Plugin and reconcile

- [x] 2.1 Add `tauri-plugin-autostart` pinned at `=2.5.0` to `src-tauri/Cargo.toml`, with a comment saying why it is pinned, and register `tauri_plugin_autostart::Builder::new().build()` beside the other plugins in `run`; verify `cargo build` succeeds, `cargo tree -i auto-launch` shows the 0.5 line, and the lockfile diff only adds entries
- [x] 2.2 Create `src-tauri/src/autostart.rs` with `apply(app, enabled)`: returns early with the development-build log line under `cfg!(debug_assertions)`, calls `enable` when on, calls `disable` only when `is_enabled` is true when off, and maps errors to the two log strings from design.md; verify with `cargo clippy -- -D warnings` and a unit test for the message formatting
- [x] 2.3 Call `apply` in `setup` after the config loads and push the tray notice `Autostart couldn't be applied, see dango.log` on an error; verify a release build on the current machine with the setting on creates the entry and with the setting absent creates nothing
- [x] 2.4 In `apply_config_reload`, read the previous `launcher.autostart` before `file_config.replace`, call `apply` only when the effective value changed, and on error log and `set_config_status(app, true)`; verify by toggling the key in `config.json` while a release build runs and watching the entry appear and disappear without a restart, and that an unrelated edit leaves it alone

## 3. Documentation

- [x] 3.1 Add to the README's release install section that macOS records the launch path, so the app must be in `/Applications` before `launcher.autostart` is turned on, and that macOS shows a one-time background-items notification; verify the text names the setting as it appears in the example config

## 4. Windows verification

- [ ] 4.1 Install a release build from the installer, set `launcher.autostart` to true, and verify `reg query HKCU\Software\Microsoft\Windows\CurrentVersion\Run /v Dango` shows the installed path, Settings > Apps > Startup lists Dango, and no value exists under the same key in `HKLM`
- [ ] 4.2 Sign out and in; verify Dango is running with its tray icon, no launcher window is visible, and the launcher hotkey works
- [x] 4.3 Set the key to false and save; verify the `Run` value is gone without a restart (verified against the local release build in `target/release`, same binary the installers wrap)
- [ ] 4.4 Turn Dango off in Settings > Apps > Startup with the config on, sign out and in, verify Dango did not start; start it by hand and verify the startup toggle is on again
- [x] 4.5 Run `tauri dev` with the setting on; verify no `Run` value is created and `dango.log` has the development-build line

## 5. macOS verification

- [ ] 5.1 Install the `.dmg` build into `/Applications`, set `launcher.autostart` to true, and verify `~/Library/LaunchAgents/Dango.plist` exists with `ProgramArguments` pointing inside `/Applications/Dango.app` and `RunAtLoad` true, and that Dango is listed under System Settings > General > Login Items & Extensions as allowed in the background
- [ ] 5.2 Log out and in; verify Dango is running with its menu bar icon, no launcher window is visible, and the launcher hotkey works
- [ ] 5.3 Set the key to false and save; verify the plist is removed without a restart
- [ ] 5.4 Launch the app from the mounted `.dmg` with the setting on, then from `/Applications`; verify the plist path is corrected by the second launch
- [ ] 5.5 Run `tauri dev` with the setting on; verify no plist is created and `dango.log` has the development-build line
- [ ] 5.6 Make `~/Library/LaunchAgents` unwritable, start a release build with the setting on; verify the tray shows `Autostart couldn't be applied, see dango.log` and the log names the write that failed, then restore permissions

## 6. Wrap up

- [x] 6.1 Review the diff for comments that restate the code, for any frontend or capability change, and for scope beyond the proposal; delete what is found and verify `cargo clippy -- -D warnings` and `cargo test` pass
- [ ] 6.2 Update the M7 section of `openspec/ROADMAP.md` to record autostart as shipped with what was verified on each platform; verify the text matches the ticked tasks above
