# Proposal

## Why

External monitors are dimmed and brightened with their own menu buttons, which is slow and awkward. Most monitors accept brightness changes over DDC/CI, so Dango can step brightness from the keyboard or a hotkey, the same way it already tiles windows.

## What Changes

- New built-in `display` extension with two no-view commands, `Increase Brightness` and `Decrease Brightness`. Both can be bound to a hotkey like any command.
- Each command changes every external monitor that answers DDC/CI by 10% of that monitor's range, from its own current level, clamped to 0% and 100%.
- After the change the notice reports the new level. When no monitor answers, or every monitor rejects the change, the command fails with a message that says so.
- A no-view command can report success with a short message, shown in the notice once the launcher hides. Brightness is the first user; until now a no-view success showed nothing.
- Platforms: Windows and macOS, Intel and Apple Silicon. External monitors only.

## Non-goals

- Built-in laptop or MacBook panels. They are not reached over DDC/CI; Windows needs WMI and macOS a private framework.
- A view listing monitors with their levels, per-monitor control, or setting an exact level. Possible later change.
- Targeting only the monitor under the focused window.
- Contrast, input source, volume, or any other VCP code.
- A configurable step size.
- Restoring brightness after sleep, schedules, or following the time of day.
- Linux (out of scope for the project).

## Capabilities

### New Capabilities

- `display-brightness`: the `display` extension's brightness step commands, which monitors they reach, how levels change, and what they report.

### Modified Capabilities

- `view-protocol`: a no-view command can report success with a one-line message.
- `command-invocation`: a superseded no-view command's success message or failure still reaches the notice, so quick brightness presses show each level as it lands.
- `notice`: the notice shows the success message of a no-view command that reported one, since a brightness level is an outcome the user cannot read off the screen.

## Impact

- New dependencies: `ddc` 0.2, `ddc-winapi` 0.2 (Windows only), `ddc-macos` 0.2 (macOS only).
- New platform trait for monitor brightness in `src/platform/`, implemented in `platform/windows` and `platform/macos`.
- New `src/extensions/display/`, registered in `src/extensions/mod.rs` and `src/app/startup.rs`.
- `src/invocation.rs` `Outcome::Success` gains an optional message; `src/app/mod.rs` shows it in the notice.
- Config schema and settings reference gain the `display` extension and its command hotkeys (generated from the manifest).
