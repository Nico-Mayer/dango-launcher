## Why

Dango is a resident tray application that is only useful while it is running,
and today it has to be started by hand after every login on both machines. M7
in `openspec/ROADMAP.md` lists autostart as one of its three items; the release
workflow is done, so this is the next one.

## What Changes

- A new `launcher.autostart` setting in `config.json`, off by default, that
  registers Dango to start at login on the current machine.
- Dango reconciles the login entry with the setting at startup and whenever the
  config file is reloaded, so turning it on or off is an edit to the file and
  nothing else. The entry is re-asserted on every startup so it follows the
  installed location.
- Only a release build registers itself. A development run never points the
  login entry at a build directory.
- A failure to register or remove the entry is surfaced the way other startup
  problems are: a disabled tray item that names `dango.log`, with the detail in
  the log.
- Documentation: `docs/config.example.jsonc` gains the setting, the README's
  release notes gain the one-time macOS consequence (the app must be in
  `/Applications` before the setting is turned on, since the entry records the
  path it was launched from).

Milestone: M7 - polish. Platforms: Windows and macOS, both covered.

## Non-goals

- Any UI for the setting. The config file is the settings surface (M5
  decision). No tray toggle, no first-run prompt.
- Starting hidden versus visible. Dango always starts hidden; nothing changes.
- Launching the app via a login-item entry that appears under the user's name
  in System Settings on macOS. The launch agent is what the plugin does by
  default and it is enough; see design.md for why.
- Per-machine overrides inside the shared config file. The setting is one
  boolean; a machine that must not autostart can leave it out of its copy or
  use `$DANGO_CONFIG_DIR`.
- Elevated or machine-wide registration on Windows. Per-user only.
- Anything about updates or relaunching after an install.

## Capabilities

### New Capabilities

- `autostart`: starting Dango at login from a config setting, how the entry is
  kept in step with the file, and how a failure is reported.

### Modified Capabilities

None. The `configuration` spec already states that a value in the file wins
over the default and that edits apply live; the new setting follows those
requirements without changing them. The `interface-copy` tray-notice
requirement covers the new notice as written.

## Impact

- `src-tauri/Cargo.toml`: new dependency `tauri-plugin-autostart`, pinned at
  2.5.0 because 2.6.0 and later require Tauri 2.12; it pulls `auto-launch` 0.5.
- `src-tauri/src/config/mod.rs`: `Launcher` gains an `autostart: Option<bool>`.
- `src-tauri/src/lib.rs`: plugin registration, one reconcile call at startup and
  one in `apply_config_reload`, one new tray notice.
- `docs/config.example.jsonc`, `README.md`: documentation.
- No change to the extension manifest, the view protocol, the frontend, or the
  database.
