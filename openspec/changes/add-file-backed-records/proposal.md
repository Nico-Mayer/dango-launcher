## Why

Part of **M5 - keys**, following `add-config-file`. That change moved
configuration into `~/.config/dango/config.json` so it travels in the user's
dotfiles. Snippets and quicklinks are the other half of what the author writes by
hand and wants on both machines, and they are still in SQLite, which is binary,
merge-hostile, and unshareable. A snippet created on the laptop should appear on
the desktop after a `git pull`, and that is exactly what M9's file-based sync is
meant to deliver; this brings the authored content forward now, as plain files,
because the config directory it belongs in already exists.

The split is deliberate: what the user authors becomes text in the dotfiles;
what the machine accumulates stays local. Clipboard history, frecency, and window
state are high-churn and private, so they remain in SQLite and never enter git.

## What Changes

- Snippets and quicklinks move from the SQLite `snippets` and `quicklinks` tables
  into text files under `~/.config/dango/` (honouring `$DANGO_CONFIG_DIR`): one
  file per kind, `snippets.json` and `quicklinks.json`, each an array of records.
- A record is `{ id, name, body }` (the body is the template for a snippet, the
  URL for a quicklink). Timestamps and the soft-delete marker are dropped:
  git is the history, so removing a record removes its entry, and a hand-authored
  entry may omit `id`, which Dango fills in on its next write.
- The `Records` store is reworked from a SQLite-backed struct to a file-backed
  one behind the same shape, so the snippets and quicklinks extensions are
  otherwise unchanged.
- The files are human-editable and edits apply live, the way `config.json` does:
  adding, changing, or deleting a record in the file shows up in the launcher
  without a restart.
- A malformed records file is reported and the last good set is kept, rather than
  losing records or crashing.
- A migration drops the `snippets` and `quicklinks` SQLite tables.

## Capabilities

### New Capabilities

- `records-storage`: where snippets and quicklinks live, their file format, how a
  record is identified, how the files are edited by hand and by the app, how edits
  apply live, what deletion means, and how a bad file is handled.

### Modified Capabilities

None. The `snippets` and `quicklinks` capabilities keep their observable
behaviour: creating, editing, removing, and finding records, and their surviving
a restart, all still hold. Only where the records live changes.

## Impact

- **Affected code**: `src-tauri/src/extensions/snippets/store.rs` (the `Records`
  store, reworked to files), `src-tauri/src/lib.rs` (build the store against the
  config directory instead of the database, and watch the files), and the SQLite
  migrations (drop the two tables).
- **Config directory**: gains `snippets.json` and `quicklinks.json` beside
  `config.json`, all git-friendly text.
- **Data**: clipboard history, frecency, and window state are untouched and stay
  in the machine-local SQLite.
- **Migration**: the `snippets` and `quicklinks` tables are dropped. Any records a
  user already had in SQLite would need a one-time export; the design says whether
  that is worth doing given how new the feature is.
- **Coordination**: `add-keyword-expansion` (in flight) also touches the snippets
  store to add a keyword. Whichever lands second reconciles the record shape; the
  file format has room for the extra field.
- **View protocol**: unchanged.

## Non-goals

- **No change to the create, edit, remove, or search flows.** The launcher forms
  and root-search behaviour are exactly as they are; only storage changes.
- **No moving machine-local data to files.** Clipboard history, frecency, and
  window state stay in SQLite.
- **No general sync engine.** This is file storage the user can commit, not a
  merge or conflict-resolution system. That is M9.
- **No new record types.** Only snippets and quicklinks, the two that exist.
- **No Linux.** Out of scope for the project.
