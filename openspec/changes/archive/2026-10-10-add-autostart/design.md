## Context

See proposal.md for the motivation. What shapes the approach:

- Settings come from `config.json`, loaded into `config::Config` at startup and
  re-applied live by `apply_config_reload` in `src-tauri/src/lib.rs`. The
  `Launcher` struct is where launcher-wide settings sit; it has only `hotkey`
  today.
- Startup problems are surfaced as disabled tray items built in `setup`, and
  reload problems through `set_config_status` plus `dango.log`. There is no
  other notification surface and this change adds none.
- Dango is never elevated on Windows (a hard ceiling recorded in the roadmap),
  starts hidden by design, and is a `LSUIElement` app on macOS.
- `tauri.conf.json` has `productName` `Dango`, which is what
  `package_info().name` returns and therefore what the plugin names its entry.
- Development runs are `tauri dev` from the repository; the binary lives under
  `src-tauri/target`. Release builds come from the installers of the release
  workflow, so the installed path differs per platform and per installer kind
  (MSI and NSIS on Windows, `/Applications/Dango.app` on macOS).

## Goals / Non-Goals

**Goals:**

- One boolean in the file, reconciled with the OS on startup and on reload, with
  no new state kept anywhere else.
- The login entry always points at the installed application that last ran
  with the setting on.
- Failures are reported in the places the user already looks.

**Non-Goals:**

- Choosing between a launch agent and a login item at runtime, or exposing that
  choice in the config.
- Detecting that the user removed the entry by hand and re-creating it while
  Dango is running.
- A frontend command. Nothing in the webview needs to know about autostart, so
  no capability permission is added.

## Decisions

### Use `tauri-plugin-autostart`, pinned at 2.5.0, called from Rust only

The official plugin is the project's default for a general job, and this is
exactly its job. It wraps `auto-launch`: on Windows a string value named after
the app under `HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Run`, plus the
`StartupApproved` flag the Settings app reads; on macOS a plist at
`~/Library/LaunchAgents/Dango.plist` with `RunAtLoad`. The plugin adds what we
would otherwise write ourselves: it resolves `current_exe` at startup,
canonicalises it inside the `.app` on macOS, and exposes `enable`, `disable`,
and `is_enabled` on the app handle through `ManagerExt::autolaunch`.

The pin: 2.6.0 and later require Tauri 2.12, and resolving them moves `tauri`,
`tao`, `wry`, and `webview2-com` a minor version each, a 440-line lockfile
change that also dropped the transitively enabled `Win32_System_Variant`
feature the Windows app indexer relies on. 2.5.0 resolves against the locked
Tauri 2.11.5 with additions only. It has the same `Builder` API and uses
`auto-launch` 0.5, which on Windows writes `HKCU` only, where 0.6 tries `HKLM`
first. The pin is explained in `Cargo.toml`; it moves when Tauri does.

Two `auto-launch` 0.5 details the reconcile accounts for: `disable` on Windows
fails when the value is already gone, so it is only called when `is_enabled`
says there is something to remove; and the Windows value is the unquoted path,
which `CreateProcess` resolves correctly for the install directories the two
installers use, verified in the Windows tasks.

Alternatives considered:

- `auto-launch` directly. Same backend without the plugin's path handling.
  Nothing it offers is needed. Rejected.
- Hand-rolled registry and plist writes. Rejected by the off-the-shelf rule;
  nothing the crate does is wrong for us.
- `SMAppService` on macOS for a first-class login item. Would need an
  Objective-C bridge and a signed bundle to behave well; the project does not
  sign. Rejected.

### Launch agent on macOS, not an AppleScript login item

The plugin's default `MacosLauncher::LaunchAgent` writes a plist and nothing
else. The `AppleScript` mode drives System Events, which triggers an Automation
permission prompt the first time and adds a second permission to the macOS
onboarding story that the next M7 change has to tell. A launch agent needs no
permission, is removed by deleting one file, and macOS 13 and later lists it
under the items allowed to run in the background, where the user can also turn
it off. Good enough for the author's two machines.

### A debug build never touches the entry

The reconcile is gated on `cfg!(debug_assertions)`. The plugin registers
`current_exe` as the path, so a `tauri dev` run with the setting on would
register `src-tauri/target/debug/dango`, and the next login would start a
stale debug binary, or nothing. The gate logs one line and returns. The plugin
itself is still registered in debug builds; its setup only resolves a path.

Alternative: gate on an environment variable or a config flag. Rejected; the
build profile is the fact that matters and it cannot be set wrong.

### Reconcile rules: on means assert, off means remove if present

On startup with the setting on, `enable` is called unconditionally. It is
idempotent and cheap (one registry value, one small file overwrite), and it is
what makes the entry follow the installed path after an install location
changes or after the app is moved from `Downloads` into `/Applications` on
macOS. On startup with the setting off or absent, `disable` is called only when
`is_enabled` is true, so a machine that never turned it on does not write
anything on every launch.

On reload, the previous value is read from the running config before
`file_config.replace`, and the OS is touched only when the value changed. A
reload that leaves the value alone does nothing, which is the spec's
"unchanged on reload" scenario and avoids re-asserting on every unrelated edit.

Consequence to be aware of: on Windows, `enable` also writes the
`StartupApproved` flag that the Settings app's startup toggle uses, so a user
who turned Dango off there while the file says on gets it turned back on at the
next manual start. The spec states this; the file is where the setting lives.

Alternative considered: track the last state Dango applied in a local file so
a Windows toggle could be respected. Rejected as a second source of truth for
one boolean on a two-machine project.

### Placement

- `Launcher` in `src-tauri/src/config/mod.rs` gains
  `autostart: Option<bool>`, included in `is_empty`.
- A new `src-tauri/src/autostart.rs` holds `apply(app, enabled) -> Result<(),
  String>` with the debug gate and the on/off rules above, so `lib.rs` gains two
  call sites and nothing else: one in `setup` after the config is loaded, one in
  `apply_config_reload` after `file_config.replace`.
- The plugin is registered next to the other plugins in `run`.

### User-facing text

Checked against `openspec/specs/interface-copy/spec.md`: names what failed,
names the log, no identifiers.

- Tray item, startup failure: `Autostart couldn't be applied, see dango.log`
- Log, debug build: `autostart: ignored in a development build`
- Log, enable failed: `autostart: could not register Dango to start at login: <error>`
- Log, disable failed: `autostart: could not remove the login entry: <error>`

`docs/config.example.jsonc`, inside `launcher`:

```jsonc
// Start Dango when you log in. Applies to the machine that reads this file.
// Default false. A development build ignores it.
"autostart": true
```

## Risks / Trade-offs

- [The macOS entry records the path the app was launched from] → If the first
  run with the setting on happens from the mounted `.dmg`, the plist points at
  a volume that will not exist at login. The README's install step already
  says to copy the app to `/Applications`; the autostart note is added there,
  and the startup re-assert repairs the entry the first time Dango is launched
  from its final place.
- [The Windows startup toggle leaves the `Run` value in place] → With the
  toggle off and the config then turned off, `is_enabled` is already false, so
  the value stays. Windows will not run it, and the next time the config turns
  on the value is rewritten. Harmless, noted so nobody hunts for it. In the
  other direction, `disable` removes the `Run` value but not the
  `StartupApproved` flag; an orphaned flag has no effect and is reused by the
  next `enable`.
- [`is_enabled` on macOS only checks that the plist exists] → A plist the user
  edited or that points at a moved app still reads as enabled. The startup
  re-assert overwrites it whenever the setting is on, which covers the moved
  case; a hand-edited plist is the user's.
- [macOS shows a one-time "added items that can run in the background"
  notification] → Expected system behaviour, not something Dango can suppress.
  Noted in the README.
- [Two machines share one file but only one should autostart] → Out of scope
  by the proposal. `$DANGO_CONFIG_DIR` or a per-machine copy is the answer.

## Migration Plan

Additive. An older Dango ignores the key and preserves it on write-back, by the
configuration spec. Rollback is removing the key; the next start with the key
absent removes the entry.
