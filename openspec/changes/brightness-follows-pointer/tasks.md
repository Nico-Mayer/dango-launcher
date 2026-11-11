# Tasks

## 1. Setting

- [ ] 1.1 Add `DisplayTable` in `src/config/tables.rs` with `enabled`, `commands`, and `monitors` (enum `All` default / `Pointer`, lowercase, doc comment from design.md), add it to `extension_table!`, and use it for `Config::display`. Regenerate `docs/config.md` and the schema (`env DANGO_UPDATE_DOCS=1 cargo test the_settings_reference_is_current`). Verify: new tests in `src/config/mod.rs` cover absent = `All`, `"pointer"` parsing, and `"focused"` failing with an error naming `monitors`; `cargo test` passes

## 2. Platform lookup

- [ ] 2.1 Add `under_pointer()` to `BrightnessMonitors` in `src/platform/mod.rs`; give the test fake in `src/extensions/display/` and `src/app/tests.rs` a pointer index. Verify: `cargo build` and `cargo clippy --all-targets -- -D warnings` pass
- [ ] 2.2 Windows: implement `under_pointer()` in `platform/windows/brightness.rs` with `GetCursorPos`, `MonitorFromPoint(.., MONITOR_DEFAULTTONULL)`, `ddc_winapi::get_physical_monitors_from_hmonitor`, and `ddc_winapi::Monitor::new`. Verify: `cargo build` and clippy pass on Windows
- [ ] 2.3 macOS: add the `CGDirectDisplay` feature to `objc2-core-graphics` and implement `under_pointer()` in `platform/macos/brightness.rs` with `CGEvent::location`, `CGGetDisplaysWithPoint`, and `ddc_macos::Monitor::enumerate()` filtered by `handle().id`. Check symbol names against the vendored crate source under `~/.cargo/registry`. Verify: CI's macOS job builds after the author pushes (left open until a macOS session)

## 3. Extension reads the setting

- [ ] 3.1 Pass `FileConfig::shared()` into `DisplayExtension::new` (`src/app/startup.rs`, tests), read `display.monitors` once per step, and call `external()` or `under_pointer()`. Verify: tests in `src/extensions/display/` cover the `display-brightness` scenarios "Default changes every monitor", "Only the monitor under the pointer changes", "Moving the pointer moves the target", "The pointer is on a monitor without DDC/CI", and "The setting applies without a restart" (change the shared config between two presses)

## 4. Hand checks

- [ ] 4.1 Windows: on the author's two monitors with `display.monitors = "pointer"`, press and hold the brightness hotkeys (`mise run keys`) with the pointer on each monitor in turn, and check only that monitor changes and the notice shows its level (`mise run shot`); switch back to `"all"` without restarting and check both change
- [ ] 4.2 Windows: on a laptop with an external monitor, pointer on the laptop screen gives the DDC/CI failure and changes nothing
- [ ] 4.3 macOS: on an Apple Silicon Mac with an external monitor, pointer on the external monitor changes it; pointer on the built-in screen gives the DDC/CI failure and changes nothing

## 5. Wrap-up

- [ ] 5.1 Add any new platform trap or build fact found in tasks 2 to 4 to AGENTS.md. Verify: `mise run check` and `cargo test` pass
- [ ] 5.2 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context; report findings to the author

## Workflow follow-up

- Archive after `display-brightness`: this change's delta modifies a requirement that only exists once that change is archived.
- Archive only after the macOS tasks 2.3 and 4.3 are done in a macOS session.
