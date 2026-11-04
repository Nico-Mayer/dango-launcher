# Proposal

## Why

The launcher and the notice always open on the display under the mouse
pointer. With the pointer left on a side monitor, the launcher opens there
even when the user works on the main one. A setting that pins both to the
primary display fixes this.

## What Changes

- New config key `launcher.display` with two values: `"pointer"` (default,
  today's behaviour) and `"primary"`.
- With `"primary"`, the launcher opens on the primary display, whatever
  display the pointer is on.
- The notice follows the same setting, so it stays on the display the
  launcher would open on.
- A changed value applies to the next show without a restart.
- The key appears in the generated schema and in `docs/config.md`.

Platforms: Windows and macOS. "Primary" is the display Windows calls primary
and, on macOS, the display with the menu bar.

## Non-goals

- A third value for the display of the active window. Possible later as one
  more enum value, not part of this change.
- A choice of a specific display by name or index.
- Remembering a display per command or per window position.
- Any change to window management commands (move to next display, tiling).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `launcher-shell`: launcher positioning depends on `launcher.display`; a new
  requirement defines the setting.
- `notice`: the notice uses the display chosen by `launcher.display`, not
  always the one under the pointer.

## Impact

- `src/config/mod.rs`: new `Display` enum and `launcher.display` field.
- `src/shell.rs`, `src/slint_shell/`, `src/app/`: the chosen display reaches
  launcher and notice placement.
- `src/platform/windows/mod.rs`, `src/platform/macos/mod.rs`: pick the work
  area of the primary display or of the display under the pointer.
- `docs/config.md` and the config schema, regenerated.
