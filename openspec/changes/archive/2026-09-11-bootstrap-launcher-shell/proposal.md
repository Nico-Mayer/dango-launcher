## Why

Milestone M0 in `openspec/ROADMAP.md`.

Dango is currently an unmodified `create-tauri-app` scaffold: a normal 800x600
window with a greet button. Before any feature exists, the application needs the
one behaviour that defines a launcher - press a key anywhere, and a prompt
appears instantly over whatever you were doing.

This has to come first because the latency budget is not something that can be
retrofitted. If the window is created on demand, or the webview boots per
invocation, every later feature inherits a launcher that feels slow. The window
lifecycle decided here constrains M1 onwards, so it is proved with a measurement
before anything depends on it.

This change also stands up the Windows and macOS CI matrix. Development happens
on macOS and daily use happens on Windows, so a Windows-only break must fail in
minutes, not in a week.

## What Changes

- Replace the default window with a borderless, transparent, always-on-top
  launcher window that is created hidden at startup and never destroyed.
- Add a system tray icon as the only persistent UI, with a menu offering toggle,
  and quit.
- Register one global shortcut that toggles launcher visibility, working while
  any other application has focus.
- Remove dock and taskbar presence so the launcher never appears in the app
  switcher and never steals activation on show.
- Position the window on the monitor holding the active window, horizontally
  centred and offset above vertical centre.
- Enforce single instance. A second launch signals the running instance to show
  rather than starting a second process.
- On hide, notify the frontend to reset to root state, since the webview is warm
  and keeps state between invocations.
- Add a minimal frontend placeholder: an empty prompt that renders and paints,
  with no search behaviour. It exists to make the latency measurement honest.
- Add a latency instrumentation path that records hotkey-to-first-paint and
  exposes it in development builds.
- Add GitHub Actions building and testing on `windows-latest` and
  `macos-latest`.
- **BREAKING** for the scaffold: the `greet` command, the default window config,
  and the template frontend are removed.

### Non-goals

- No search, no results list, no commands of any kind. M0 shows an empty prompt.
- No extension registry, no manifest, no view protocol. That is M1.
- No SQLite, no persistence. The shortcut is a hardcoded default.
- No preferences window and no way to rebind the shortcut from the UI.
- No autostart, no code signing, no updater. Deferred to M7 or dropped.
- No Linux. The author owns no Linux desktop.

### Platforms

Windows and macOS, both required. Linux is out of scope for the project.

The two platforms need genuinely different window implementations. macOS needs
an NSPanel to appear over fullscreen applications without activating the app,
which a standard Tauri window cannot do. Windows needs a topmost tool window
that is excluded from the taskbar and from Alt+Tab. This change is the first
exercise of the platform trait boundary that the rest of the project depends on.

## Capabilities

### New Capabilities

- `launcher-shell`: the resident tray application, the hidden-and-shown launcher
  window, its lifecycle and positioning, and single-instance behaviour.
- `global-activation`: registering and handling the global shortcut that
  summons the launcher, including failure when the combination is already taken
  by another application.

### Modified Capabilities

None. This is the first change in the project.

## Impact

- `src-tauri/src/`: new modules for tray, window control, activation, and a
  `platform` module split into `macos` and `windows` behind a window-behaviour
  trait.
- `src-tauri/tauri.conf.json`: window definition becomes hidden, borderless,
  transparent, always-on-top, skip-taskbar.
- `src-tauri/Cargo.toml`: adds `tauri-plugin-global-shortcut`,
  `tauri-plugin-single-instance`, and a macOS NSPanel dependency.
- `src-tauri/capabilities/default.json`: permissions for the added plugins.
- `src/`: template route replaced by the placeholder prompt. Decision pending in
  design on whether SvelteKit stays.
- New `.github/workflows/`: two-platform build and test matrix.
- Removed: the `greet` command and the scaffold's demo frontend.
