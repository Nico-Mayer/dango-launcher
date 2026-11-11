# Proposal

## Why

The brightness commands change every external monitor at once. With monitors in different light, or one used for something else, the user wants to change only the monitor they are looking at. The mouse pointer is the cheapest signal for that, and the launcher already uses it to pick its display (`launcher.display = "pointer"`).

## What Changes

- New setting `display.monitors` in the config file: `"all"` (default, today's behaviour) or `"pointer"`.
- With `"pointer"`, `Increase Brightness` and `Decrease Brightness` change only the external monitor under the mouse pointer, read when the step runs.
- When the monitor under the pointer does not answer DDC/CI (DDC/CI off, or a built-in laptop panel), the command fails with the existing "no monitor answers" message. It does not fall back to other monitors.
- Changing the setting takes effect on the next press, after the usual config reload. No restart.
- Platforms: Windows and macOS.

## Non-goals

- Targeting the monitor under the focused window instead of the pointer.
- A per-command or per-hotkey choice (one hotkey for all monitors, another for the pointer's). One setting for the extension.
- Picking monitors by name or index in the config.
- Brightness of built-in panels (still out of scope, see `display-brightness`).
- New failure text for the pointer case. The existing messages are reused.
- Linux (out of scope for the project).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `display-brightness`: which monitors a step changes now depends on the `display.monitors` setting. This capability is still only in the open `display-brightness` change, so this change archives after it.

## Impact

- `src/config/tables.rs` and `src/config/mod.rs`: `display` gets its own table type with a `monitors` field; config schema and `docs/config.md` regenerated.
- `src/platform/mod.rs`: `BrightnessMonitors` gains a way to list only the external monitor under the pointer.
- `src/platform/windows/brightness.rs`: `GetCursorPos` + `MonitorFromPoint`, then `ddc_winapi::get_physical_monitors_from_hmonitor`.
- `src/platform/macos/brightness.rs`: pointer location from CoreGraphics, matched against `ddc_macos::Monitor::handle()`.
- `src/extensions/display/`: reads the setting from shared config on each step.
- No new crate. No manifest or view protocol change.
