## Context

See proposal.md - Why for the motivation. What shapes the approach:

- `src-tauri/src/hotkeys.rs` builds a first-wins table from
  `extensions.<id>.commands.<id>.hotkey`. Each `Binding` is a `Shortcut` plus a
  qualified command id string. `lib.rs` registers the survivors with the
  global shortcut plugin, holds them as `Vec<(Shortcut, String)>`, and on a
  press calls `dispatch_command(app, &command_id)`, which checks the command's
  mode in the registry and runs it through `run_command`.
- `apply_command_hotkeys` in `lib.rs` runs at startup and on every config
  reload: it unregisters the previous table, rebuilds, registers, and returns
  conflicts that go to the tray status line and `dango.log`.
- The applications extension declares no commands. Apps are `IndexedApp {
  id, name, target }` in an `AppIndex` that is loaded from the store at
  startup and rebuilt in the background. `id` is a bundle path on macOS and an
  AppUserModelID on Windows, so it is never portable. `name` is what root
  search shows: the localised display name on macOS, the shell's name on
  Windows. `ApplicationsExtension::perform(app_id, ACTION_LAUNCH)` launches,
  brings a running app forward, and evicts a vanished entry. `lib.rs` keeps the
  extension in Tauri state.
- `run_action` in `lib.rs` credits frecency for any `Done` action, so a launch
  from root search counts as use.
- `config::Extension` is `enabled`, `preferences`, `commands`, and a flattened
  `extra` map that preserves unknown keys. `Hotkey` is a string or a
  `{ macos, windows }` object, resolved for the running platform by
  `Hotkey::parse_with_hyper`.

## Goals / Non-Goals

**Goals:**

- One binding table for commands and applications, so conflicts are decided in
  one place with one rule.
- A config entry that reads the same on both machines, resolved per platform
  where the platforms genuinely differ.
- Reuse the launch path root search already uses, so a hotkey launch cannot
  drift from an Enter launch.

**Non-Goals:**

- A generic "bind any root item" facility. Applications are the one provider
  whose items are stable enough to name from a file.
- Resolving the application at config time. The index changes at run time and
  the file is shared across machines, so the name is resolved at the press.

## Decisions

### The entry is keyed by the shown name, with a per-platform override

```jsonc
"dango.applications": {
  "apps": {
    "Safari": { "hotkey": "hyper+b" },
    "editor": {
      "name": { "macos": "TextEdit", "windows": "Notepad" },
      "hotkey": "hyper+e"
    }
  }
}
```

The key is the name unless a `name` field says otherwise. `name` is a string
or a `{ macos, windows }` object, the same shape `Hotkey` already uses, so the
file has one way of saying "this differs per platform". A `name` object with
no value for the running platform means the entry is not for this machine and
registers nothing.

Matching is exact, ignoring case, against `IndexedApp.name`. The name is the
one identifier the user can read off the launcher on both platforms without
tooling; a bundle id or AppUserModelID needs a terminal to discover.

Alternatives rejected:

- Key the map by chord (`"hotkeys": { "hyper+b": "Safari" }`). Per-platform
  chords then need an object as a key, which JSON cannot express.
- Bind by launch id, the bundle path or AppUserModelID. Exact and unambiguous,
  but unreadable and never portable, so every entry would need two values.
- Fuzzy match the name the way root search does. A hotkey has no list to pick
  from, so a near miss would launch the wrong application silently.

### `apps` is a typed field on the extension config

`config::Extension` gains `apps: BTreeMap<String, AppSettings>` with
`AppSettings { name: Option<AppName>, hotkey: Option<Hotkey>, extra }`, next to
`commands`. Only the applications extension reads it, but typing it in the
config model is what lets `hotkeys.rs` iterate it alongside `commands` without
re-parsing JSON, and the schema and example document it the same way.

Alternatives rejected:

- Parse `apps` out of the flattened `extra` map inside the applications
  extension. Keeps the config model generic, but the binding table would then
  be built in two places with two conflict rules, or the extension would have
  to hand its bindings back to `hotkeys.rs` in a second pass.
- Make the applications extension declare one virtual command per bound app
  so the existing `commands` path applies. Commands are static manifest
  declarations, and the registry, root search, and aliases would all see the
  virtual entries.

### A binding's target is an enum

```rust
pub enum Target {
    Command(String),
    Application(String),
}
pub struct Binding { pub chord: Shortcut, pub target: Target }
```

`command_bindings` becomes `bindings` and iterates `commands` then `apps` per
extension, in the config's stable order, so first-wins is deterministic and a
command and an application competing for a chord are reported by name. The
registered table in `lib.rs` holds `(Shortcut, Target)`. Conflict messages
name an application as `application "Safari"` so they read differently from a
qualified command id.

### Dispatch resolves the name at the press

On `Target::Application(name)`, `lib.rs` asks the managed
`ApplicationsExtension` for `find_by_name(&name)`, a case-insensitive scan of
`AppIndex::snapshot()` sorted by name. With a match it calls
`host.perform_action(EXTENSION_ID, &app.id, ACTION_LAUNCH, &FormValues::new())`,
the same call `run_action` makes for Enter, and on `Done` credits frecency and
hides the launcher if it happens to be showing. With no match, or on `Failed`,
the message goes through `log_config` and the tray status, which is where a
headless failure is already visible. Two matches launch the first and report
the ambiguity the same way.

Resolving at the press rather than at registration is what makes a shared file
work: the chord is registered on a machine where the app is missing, and starts
working the moment the app is installed and indexed, with no reload.

Alternatives rejected:

- Resolve at registration and skip unknown names. Cheaper per press, but the
  binding would go stale when an app is installed later, and a fresh machine
  would need a config touch after every install.
- Show the launcher with the failure message. The user did not ask for the
  launcher, and a headless command's failure does not show it either.

### The launcher is not shown for an application

`dispatch_command` shows the launcher for a view command. An application launch
never has a view, so `Target::Application` takes the headless branch: on
Windows the foreground window is remembered first, as for a headless command,
because the launch path's focus handling expects it.

## Risks / Trade-offs

- A name match can be ambiguous, two apps called "Code" for example → the first
  in name order launches and the tray says so, so the user can add a
  distinguishing `name`.
- A localised macOS display name may differ from what the user types → the
  match is against the name root search shows, and the tray message on a miss
  quotes the name that was looked for.
- The `AppIndex` snapshot is cloned per press → the index is a few hundred
  entries and a press is a user action, so this is well under any budget.
- `apps` on a non-applications extension is accepted and ignored → the schema
  scopes it under `dango.applications` in the example, and nothing else reads
  it.

## Migration Plan

None. The `apps` branch is new and optional. An older Dango preserves it as an
unknown key. Rollback is reverting the commit.
