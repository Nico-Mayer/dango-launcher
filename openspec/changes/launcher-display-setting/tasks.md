# Tasks

## 1. Config key

- [x] 1.1 Add `LauncherDisplay { Pointer, Primary }` and `launcher.display` to `src/config/mod.rs` with the doc comment from design.md, and include it in `Launcher::is_empty`; verify with config unit tests that `display = "primary"` parses, an absent key is `None`, and `display = "secondary"` fails with an error naming the setting (`cargo test config`)
- [x] 1.2 Regenerate `docs/config.md` with `env DANGO_UPDATE_DOCS=1 cargo test the_settings_reference_is_current`; verify `the_settings_reference_is_current` passes and the schema test rejects `"secondary"` for `launcher.display`

## 2. Core pushes the display to the shell

- [ ] 2.1 Add `Shell::set_display(LauncherDisplay)`, implement it in `RecordingShell`, and call it from Core at start before `set_event_handler` and in `apply_config_reload`; verify with app tests that start records `Pointer` when the key is absent and that a reload setting `"primary"` records `Primary` (`cargo test app`)
- [ ] 2.2 Store the value in `SlintShell` (default `Pointer`) and pass it to `position_on_active_display` and `NoticeWindow::place`, updating the test fakes in `src/slint_shell/tests.rs` (the `LauncherWindow` fake and `NoPlatform`); both platform impls ignore it for now, so behaviour is unchanged; verify `cargo test` and `mise run check` pass

## 3. Windows

- [ ] 3.1 In `src/platform/windows/mod.rs`, pick the monitor by `LauncherDisplay` (`MonitorFromPoint` at (0, 0) with `MONITOR_DEFAULTTOPRIMARY` for `Primary`) for both launcher and notice; verify `mise run check` passes
- [ ] 3.2 Hand check on Windows with two monitors, debug build via `dango-ui`, `mise run keys` and `mise run shot`: with `display = "primary"` and the pointer on the secondary monitor, the launcher and a notice appear on the primary monitor; switching to `"pointer"` without restart moves the next show to the pointer's monitor; with mixed DPI the launcher has correct size

## 4. macOS

- [ ] 4.1 In `src/platform/macos/mod.rs`, add a `primary_screen()` helper (`NSScreen::screens()` index 0), use it for `Primary` and as the fallback of `screen_under_cursor` instead of `mainScreen`, for both launcher and notice; verify `mise run check` passes on macOS (until then, CI compile check only)
- [ ] 4.2 Hand check on macOS with two displays: with `display = "primary"` and the pointer on the secondary display, the launcher and a notice appear on the main display set in System Settings > Displays; switching to `"pointer"` without restart moves the next show to the pointer's display

## 5. Review

- [ ] 5.1 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context; verify its findings are reported to the author

## Workflow follow-up

- Leave 4.1 and 4.2 open until a session on the Mac. Archive only after they
  are done.
