## Context

See proposal.md for why. What already exists and shapes this:

- `PreferenceStore` and `EnabledStore` are traits (`extension/preferences.rs`,
  `extension/mod.rs`), each with a SQLite implementation on `Store`. Nothing
  reads those tables directly, so the backing store can be swapped for a file
  behind the same traits without touching the readers.
- Preferences are declared in the manifest (`PreferenceDecl`: key, kind, default,
  required, scoped to an extension or a command). The default lives in the
  manifest; only overrides need to live in the file.
- The launcher hotkey is hardcoded in `lib.rs` as `Shortcut::new(ALT, Space)` and
  registered through `tauri-plugin-global-shortcut`, which takes `Modifiers` and a
  physical `Code`.
- `ExtensionHost` already enables and disables extensions at runtime
  (`set_enabled`), releasing and restoring commands, root items, and services. A
  reload just drives that path.
- Command aliases exist on `CommandDecl` but are always `None`; the search
  candidates already carry an `alias` field, so a configured alias has somewhere
  to flow.
- The clipboard watcher already solved own-write suppression, the exact shape the
  config watcher needs when the app writes the file itself.

## Goals / Non-Goals

**Goals:**

- One predictable, git-friendly file that is the whole configuration surface, and
  a loader solid enough that per-command hotkeys, the hyperkey, and a future GUI
  are small additions on top.
- Never crash or silently misbehave on a bad file.
- Keep a future settings GUI possible without building one, by making writes
  lossless.

**Non-Goals:**

- See proposal.md. At design level additionally: no schema for settings that do
  not exist yet beyond reserving their key names, and no attempt to validate a
  hotkey against the OS keymap (a chord the OS rejects is a registration failure,
  handled where registration already reports failures).

## Decisions

### Configuration is a file, data stays in SQLite

The split is the whole idea: configuration is declarative, small, hand-written,
and shared; data is mutable, machine-local, and accumulated. Preferences and
enable/disable were in SQLite only because that was the store at hand, and
nothing ever wrote them. They move to the file. Clipboard history, frecency, and
window state stay in SQLite, which never enters git.

Rejected: layering the file over SQLite with precedence. Two sources of truth,
no writer for the SQLite side, and a permanent "which wins" question. One layer
is simpler and is what a dotfile is.

### The file lives at a fixed `~/.config/dango/`, not the OS config dir

The point is a path a person can find, symlink, and commit on both machines. The
OS-native dirs (`%APPDATA%`, `~/Library/Application Support`) are neither
predictable across platforms nor where dotfiles live. So the path is chosen
explicitly and is the same string on both, with `$DANGO_CONFIG_DIR` for anyone
who keeps their dotfiles elsewhere. The directory is created on first need.

### The schema addresses everything by extension, then command

```json
{
  "$schema": "https://.../dango.schema.json",
  "version": 1,
  "launcher": { "hotkey": "alt+space" },
  "extensions": {
    "dango.clipboard": {
      "enabled": true,
      "preferences": { "max_entries": 200 }
    },
    "dango.window-management": {
      "commands": {
        "left-half": { "alias": "lh" }
      }
    }
  }
}
```

Every setting has one path: `extensions.<id>.enabled`,
`extensions.<id>.preferences.<key>`, `extensions.<id>.commands.<id>.alias`. This
change reads `launcher.hotkey`, `enabled`, `preferences`, and `alias`. The
`commands.<id>.hotkey` key and a top-level `hyperkey` are reserved for the next
changes: they may appear in the file now and are simply preserved and ignored, so
a config written for a later Dango loads today.

Rejected: a flat key space like `"dango.clipboard.max_entries"`. It reads worse,
nests badly for per-command settings, and does not group an extension's settings
for a human scanning the file.

### Hotkeys are portable tokens over physical keys

A hotkey is a string: modifier tokens plus one key, `mod+shift+k`. The tokens are
platform-portable so one dotfile works on both machines:

```
  mod    Cmd on macOS, Ctrl on Windows      (the primary accelerator)
  ctrl   the literal Control key
  alt    Alt / Option
  shift  Shift
  meta   Cmd / Windows key
  hyper  the hyperkey chord (its own later change)
```

The key after the modifiers is a physical position (`Space`, `K`, `Left`), not a
character, so a non-US layout does not shift the binding; it maps to the plugin's
`Code`. When one dotfile genuinely needs to differ per platform, a hotkey may be
an object instead of a string:
`"hotkey": { "macos": "cmd+k", "windows": "ctrl+alt+k" }`. This change implements
the grammar for the one hotkey it binds, the launcher; later changes reuse it
unchanged.

Rejected: character-based keys (`cmd+ö`), which break across layouts, and
raw platform keycodes in the file, which no human should hand-write.

### File-backed stores behind the existing traits

A `Config` value is loaded once, held behind a shared lock, and read by a
`FileConfig` that implements both `PreferenceStore` and `EnabledStore`. The
readers are untouched; only the implementation handed to them changes in `lib.rs`.
On reload the held `Config` is swapped and the enable/disable and hotkey
differences are applied. The SQLite `preferences` and `extension_state` tables are
dropped from the schema.

Preference values in the file are typed JSON (a number is a number), but the
`PreferenceStore` trait speaks strings today; the file-backed store serialises the
JSON value to the string form the reader expects, so the typed readers keep
working. Widening the trait to typed values is possible later and out of scope
here.

### The loader preserves what it does not understand

To keep a future GUI-write from dropping settings, the parsed form keeps unknown
keys rather than discarding them: known fields are typed, and a catch-all holds
the rest at each level that needs forward compatibility. Writing the file back
serialises the typed fields and the preserved extras together. This is also what
lets an older Dango and a newer one, or a plugin's own settings, share one file.

Comments are not supported: JSON has none, and a write-back could not keep them.
The `$schema` reference gives editors field documentation and autocomplete
instead, which is the substitute for a settings screen.

### Live reload drives the paths that already exist

The file is watched with `notify`, debounced to coalesce an editor's multiple
saves. On a clean parse the new `Config` replaces the old, then: the launcher
hotkey is re-registered if it changed, extensions whose enabled state changed are
enabled or disabled through `ExtensionHost::set_enabled`, and preference reads
simply see the new values on next access. On a parse failure the old `Config`
stays and the error is surfaced. When Dango writes the file itself, it records
what it wrote and the watcher ignores that one change, the clipboard watcher's
own-write trick pointed at a file.

### A bad file is loud, not fatal

Configuration errors have nowhere to go without a GUI, so they go to two places:
a tray item that changes to signal a config error and reveals the message, and a
log file beside the config. Startup on a broken file falls back to defaults and
surfaces the error rather than refusing to start. This matches the project's
standing rule that failures are reported, never silent.

Rejected: refusing to start on a bad config. A typo in a dotfile should not lock
the user out of their launcher.

## Risks / Trade-offs

- **Retiring the SQLite config tables is a one-way migration.** Anything a user
  had set there would be lost. In practice nothing wrote them, so there is
  nothing to lose, but the change is stated rather than assumed.
- **No comments in the file.** A real cost for a hand-edited file. Mitigated by
  `$schema` documentation and a shipped, commented example in the repo (the
  example is documentation, not the live file).
- **The watcher can see a half-written file** if an editor writes non-atomically.
  Debouncing and keeping the last good config absorb it: a transient parse failure
  during a save does not disrupt the running app, and the settled file loads.
- **String-valued preference trait.** Serialising typed JSON to strings at the
  boundary is slightly lossy in spirit. Accepted for now; widening the trait is a
  later, isolated change.
- **Windows has no `~/.config` convention.** Using it there anyway is deliberate
  for dotfile portability, and `$DANGO_CONFIG_DIR` covers anyone who dislikes it.
