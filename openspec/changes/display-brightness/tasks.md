# Tasks

## 1. No-view success with a message

- [x] 1.1 Change `Outcome::Success` to carry `Option<String>` and add `InvocationContext::succeed_with`; update every existing `Outcome::Success` match and test. Verify: `cargo test` passes with no behaviour change
- [x] 1.2 In `src/app/mod.rs`, success with a message hides the launcher as today, then shows a success notice through a new `ui::Event` that calls `show_notice(TickerStyle::Success, ...)`. Verify: new tests in `src/ui/tests.rs` and `src/app/tests.rs` cover the `view-protocol` scenario "No-view command succeeds with a message" (from root search and from a hotkey: launcher hidden, notice text set) and that a plain success still shows no notice

## 2. Monitor access

- [ ] 2.1 Add the `BrightnessMonitors` and `Monitor` traits, `Level`, and `MonitorError` (typed, `thiserror`) to `src/platform/mod.rs`, plus `ddc = "0.2"` and one adapter generic over `ddc::Ddc` that takes the monitor's name from its constructor. `Monitor` has no `Send` bound (see design.md). Verify: `cargo build` and `cargo clippy --all-targets -- -D warnings` pass
- [ ] 2.2 Windows: add `ddc-winapi = "0.2"` under the Windows target and implement `external()` in `platform/windows` with `ddc_winapi::Monitor::enumerate()`. Verify: `cargo build` and clippy pass on Windows
- [ ] 2.3 macOS: add `ddc-macos = "0.2"` under the macOS target and implement `external()` in `platform/macos` with `ddc_macos::Monitor::enumerate()` (it already skips built-in displays). Check symbol names against the vendored crate source under `~/.cargo/registry`. Verify: CI's macOS job builds after the author pushes (left open until a macOS session)

## 3. Display extension

- [ ] 3.1 Add `src/extensions/display/` with the pure step function (tenth of each monitor's maximum, at least 1, clamped) and the message builder ("Brightness set to 60%", "Brightness set to 40% and 70%", three or more levels with commas). Verify: unit tests cover the `display-brightness` scenarios "Two monitors at different levels", "Already at the top", "Already at the bottom", "One level", "Different levels", a monitor whose maximum is not 100, and a monitor reporting maximum 0 counted as not answering
- [ ] 3.2 Add the two no-view commands. Each holds one `static Mutex<()>` for its whole step (enumerate, read, write) and never checks `is_cancelled`. Map results: no monitor answers, every monitor refuses, partial success, with each monitor error written to `dango.log`. Verify: tests with fake monitors cover "Three quick presses" (three runs, monitor ends at 80%, runs superseded by later ones still apply), "No monitor answers", "Every monitor refuses", and "One of two monitors refuses"
- [ ] 3.3 Add the manifest (ids `increase-brightness`, `decrease-brightness`, titles, keywords, icons `sun` and `sun-dim` from design.md), register the icons in `src/slint_shell/icons.rs` if missing, add the extension to `builtin_manifests` and `src/app/startup.rs`. Verify: `every_built_in_manifest_validates` passes; a root search test finds both commands for "bright", "dim", and "monitor" and none when the extension is disabled; the generated config schema includes `display.commands.increase-brightness.hotkey`

## 4. Hand checks

- [ ] 4.1 Windows: on the author's monitors, run both commands from root search and from a bound hotkey (`mise run keys`), press the hotkey quickly several times, and open the launcher during a change. Check with `mise run shot` that the notice shows the level, the launcher does not appear for the hotkey, and the launcher still opens at once; turn DDC/CI off in one monitor's menu and check the partial and "no monitor" messages
- [ ] 4.2 Windows: on a laptop with an external monitor, check that only the external monitor changes
- [ ] 4.3 macOS: on an Apple Silicon Mac with an external monitor over USB-C or DisplayPort, run both commands from a hotkey and check the level changes and the notice reports it, in a debug build and in a signed bundle (see AGENTS.md for testing a signed copy); note any permission prompt
- [ ] 4.4 macOS: on the same Mac, check the built-in panel does not change

## 5. Wrap-up

- [ ] 5.1 Add any new platform trap or build fact found in tasks 2 to 4 to AGENTS.md. Verify: `mise run check` and `cargo test` pass
- [ ] 5.2 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context; report findings to the author

## Workflow follow-up

- Archive only after the macOS tasks 2.3, 4.3, and 4.4 are done in a macOS session.
