## Why

This is the foundation of **M5 - keys** from `openspec/ROADMAP.md`, reworked. The
roadmap imagined a hotkey recorder and a settings window. Instead Dango is
configured by a JSON file the user keeps in their dotfiles and shares over git,
which fits a keyboard-first, single-author tool far better than a settings UI and
lets the whole of M5 skip building one.

The file is the crucial, careful piece the rest of M5 stands on: per-command
hotkeys, the hyperkey, and moving authored content into the dotfiles all address
their settings through it. Getting its location, its shape, its precedence over
the defaults, and its live reload right now means the changes after it are small
additions rather than reworks.

Today preferences and enable/disable live in SQLite with no way to set them,
because nothing was ever built to write them. That was always the wrong home for
configuration: it is per-machine, opaque, and unshareable. This change moves
configuration to the file and leaves SQLite for data.

## What Changes

- A single `config.json` at `~/.config/dango/` on both platforms, with a
  `$DANGO_CONFIG_DIR` override, is the source of truth for configuration.
- It carries: the launcher hotkey, per-extension enable/disable, extension
  preferences, and command aliases. Per-command hotkeys and the hyperkey have a
  reserved place in the schema but are implemented by later changes.
- The `PreferenceStore` and `EnabledStore` traits gain file-backed
  implementations reading the config, replacing the SQLite-backed ones. The
  SQLite configuration tables are retired.
- The launcher hotkey stops being hardcoded Alt+Space and is read from the file,
  falling back to the platform default when unset.
- The file is watched and applied live: an edit re-reads it and re-applies
  hotkeys, preferences, and enabled state without a restart, with the app's own
  writes suppressed the way the clipboard watcher suppresses its own.
- The loader preserves keys it does not understand, so a future settings GUI, or
  a newer Dango, or a plugin's settings can write the file without dropping
  anything. A `$schema` reference gives editors autocomplete and validation.
- A malformed file never takes the app down: the last good configuration keeps
  running and the error is surfaced through the tray and a log file rather than
  failing silently.

## Capabilities

### New Capabilities

- `configuration`: where the config file lives, its shape and versioning, how it
  is loaded, validated, and reloaded live, how its values take precedence over
  the built-in defaults, what it is the source of truth for, and how a bad file
  is handled and surfaced.

### Modified Capabilities

- `global-activation`: the summoning shortcut is read from the configuration
  file, with the existing platform defaults used when it is unset.

## Impact

- **Affected code**: a new `src-tauri/src/config/` module (schema, load,
  validate, watch); `src-tauri/src/extension/preferences.rs` and
  `src-tauri/src/extension/mod.rs` (file-backed `PreferenceStore` and
  `EnabledStore`); `src-tauri/src/lib.rs` (load config at startup, register the
  launcher hotkey from it, install the watcher, surface errors in the tray);
  `src-tauri/src/store/` (retire the `preferences` and `extension_state` tables).
- **Config directory**: `~/.config/dango/` is created if absent. The path is
  chosen explicitly rather than the OS-native config dir, because the point is a
  predictable, git-managed location shared across machines.
- **Dependencies**: a file-watching crate (`notify`) and a JSON crate already in
  the tree (`serde_json`).
- **Data**: unchanged here. Clipboard history, frecency, and window state stay in
  the machine-local SQLite. Moving snippets and quicklinks into the config dir is
  a separate change.
- **Migration**: the SQLite configuration tables are dropped. In practice nothing
  had written user values to them, since there was no interface to, so there is
  no user data to carry over; the built-in defaults apply until the file sets
  otherwise.

## Non-goals

- **No per-command hotkeys.** The schema reserves the place, but binding commands
  to global hotkeys and detecting conflicts is the next change, `add-command-hotkeys`.
- **No hyperkey.** The system-wide CapsLock remap is its own later change on the
  key monitor.
- **No moving snippets or quicklinks to files.** That refactor is its own change
  after this foundation.
- **No settings GUI.** The file is edited by hand for now. The design keeps a
  future GUI possible by making writes lossless, but builds none.
- **No comments in the file.** JSON has none, and a future GUI writing the file
  could not preserve them; `$schema` documents the fields instead.
- **No sync.** M9 still owns syncing data. This only makes configuration a file.
- **No Linux.** Out of scope for the project.
