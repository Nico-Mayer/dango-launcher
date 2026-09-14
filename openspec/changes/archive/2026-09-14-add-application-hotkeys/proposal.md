## Why

The most common thing a launcher does is open an application, and the fastest
way to open one is a single chord: hyper+B for the browser, hyper+T for the
terminal. Dango can already bind any command to a global hotkey, but
applications are not commands. They are root items from a dynamic provider,
with no stable path in the config file, so there is nothing a hotkey can name.
The only workaround is a quicklink pointing at the app bundle, which is
platform-specific and hides the app behind a different extension.

Milestone: M5 - keys. It extends the hotkey binding that milestone shipped to
the one kind of target it left out.

## What Changes

- The config file gains a place to bind applications:
  `extensions.dango.applications.apps.<name>.hotkey`, in the same hotkey
  grammar as every other binding, including per-platform chords and the hyper
  modifier.
- An application is addressed by the name Dango shows for it in root search.
  Where the name differs between platforms, the entry may carry a `name` object
  with `macos` and `windows` values, so one config file works on both machines.
- Pressing a bound chord launches the application, or brings it to the front if
  it is already running, exactly as Enter on that result would, without the
  launcher appearing.
- Application bindings take part in the existing conflict handling: first
  binding to a chord wins, whether it is a command or an application, and every
  conflict is surfaced through the tray status line and the log.
- A binding whose application is not installed is registered anyway, so the
  file can be shared between machines. Pressing it surfaces the missing
  application rather than failing silently.
- Bindings are applied on startup and re-applied on a live config edit, like
  command hotkeys.

## Capabilities

### New Capabilities

<!-- None. -->

### Modified Capabilities

- `command-hotkeys`: adds the requirement that an application can be bound to a
  global hotkey, with the conflict, live-edit, and missing-application rules.
- `applications`: adds the requirement that an application is addressable by
  its shown name, per platform, and that launching from a hotkey behaves like
  launching from root search.
- `configuration`: the source-of-truth requirement names application hotkeys
  among what the file controls.

## Impact

- `src-tauri/src/config/mod.rs`: an `apps` map on an extension's config, each
  entry with a `hotkey` and an optional per-platform `name`.
- `src-tauri/src/hotkeys.rs`: a binding's target becomes either a command or
  an application name, with the same first-wins table.
- `src-tauri/src/lib.rs`: dispatching a pressed chord to an application launch,
  and reporting a missing application.
- `src-tauri/src/extensions/applications/`: looking an application up by its
  shown name.
- `docs/config.schema.json` and `docs/config.example.jsonc`: the new branch.
- No change to the extension manifest, the view protocol, or the database.

Platforms: macOS and Windows. Launching reuses each platform's existing
indexer, so no new platform code is needed. Names are matched against what
each platform's index already shows, which is the one place they may differ,
and the per-platform `name` object covers that.

## Non-goals

- No toggling: pressing the chord while the application is frontmost does not
  hide it.
- No binding by bundle identifier, path, or AppUserModelID. The shown name is
  the one thing the user can read off the launcher on both platforms.
- No hotkey recorder or settings surface. The binding is written in the file.
- No hotkeys for other root-item providers such as snippets or quicklinks.
  Those are already reachable as commands, or can be when needed.
- No fuzzy matching of the name. It matches the shown name exactly, ignoring
  case.
