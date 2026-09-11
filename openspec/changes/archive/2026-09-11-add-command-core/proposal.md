## Why

Milestone M1 in `openspec/ROADMAP.md`.

After M0 the shortcut summons an empty prompt. This change turns that prompt into
a working launcher, and in doing so lays down the contract that every later
milestone builds on.

The ordering matters. Dango's architecture is that everything invocable is a
command, every command belongs to an extension, and built-in features are
extensions too. The value of that model comes from built-ins pressure-testing
the contract before anything external depends on it. So the contract and its
first real consumer must ship together: a manifest with nothing implementing it
is a guess, and an app launcher hardcoded outside the registry is a rewrite
waiting to happen.

`applications` is the right first consumer because it is the feature the author
will actually use daily, so the contract gets exercised under real conditions
rather than by a toy.

## What Changes

- Add the extension model: a manifest at version 1, four contribution types
  (`commands`, `rootItems`, `services`, `preferences`), and an enable and
  disable lifecycle that registers and unregisters everything an extension
  contributes.
- Add the command registry, holding every command from every enabled extension
  with its identity, keywords, and alias.
- Add the view protocol at version 1: a declarative description of list, detail,
  and form views with an attached action panel, delivered as full-tree replace.
- Add the root search pipeline: providers queried per keystroke with a time
  budget, cancel-and-restart on new input, results merged and ranked by fuzzy
  match combined with frecency, streamed to the frontend as they arrive.
- Add local persistence: SQLite with forward-only migrations, and the syncable
  record convention of UUID `id`, `updated_at`, and `deleted_at` on every table
  that could ever sync.
- Add frecency tracking so that launching an item improves its future ranking.
- Add the `applications` built-in extension: indexing installed applications on
  both platforms, extracting icons, launching, and keeping the index fresh.
- Build the frontend rendering shell: a renderer for the view protocol, a view
  stack with push and pop, keyboard navigation, and the action panel.
- Replace M0's placeholder prompt with the real one.

### Non-goals

- No third-party extensions and no extension host. Only the built-in host
  exists. Script commands and a sandboxed runtime are M8.
- No extension management UI. Enable and disable is a capability of the model,
  reachable in code and in the database, not yet a settings screen. That is M7.
- No per-command hotkeys. The manifest reserves a place for them; binding them
  is M6.
- No clipboard, snippets, quicklinks, calculator, window management, or AI.
- No grid views, no menu-bar commands, no command arguments. Added when a real
  command needs them.
- No file search. Indexing every file on disk is a different problem from
  indexing applications.
- No sync. The record convention exists so sync stays possible, nothing more.
- No Linux.

### Platforms

Windows and macOS, both required.

Application indexing is where the platforms diverge hardest and where most of
the risk in this change sits. macOS enumerates `.app` bundles and reads their
metadata. Windows must enumerate `shell:AppsFolder` to see packaged and
unpackaged applications together; scanning Start Menu shortcuts would miss every
Store application. Icon extraction and launching differ correspondingly.

## Capabilities

### New Capabilities

- `extension-model`: the extension manifest, the four contribution types, the
  enable and disable lifecycle, and the command registry that holds the result.
- `view-protocol`: the versioned declarative contract describing what a command
  renders and which actions it offers.
- `root-search`: the query pipeline from keystroke to ranked results, including
  provider budgets, cancellation, matching, and frecency.
- `local-store`: local persistence, schema migration, and the record convention
  that keeps future sync possible.
- `applications`: the built-in extension that finds, ranks, and launches
  installed applications on both platforms.

### Modified Capabilities

None. M0's `launcher-shell` requirements still hold unchanged: the reset-on-hide
signal keeps its contract and gains a concrete consumer in the view stack, and
the 80ms activation budget must survive the registry and search pipeline being
loaded. Both are verified here rather than respecified.

## Impact

- `src-tauri/src/`: new modules for the registry, extension model, view
  protocol types, search pipeline, ranking, and store. New
  `extensions/applications` module with platform-specific indexers behind an
  `AppIndexer` trait.
- `src-tauri/Cargo.toml`: adds a SQLite driver with migrations, a fuzzy matcher,
  UUID generation, and the platform crates needed for shell and bundle
  enumeration.
- `src/`: the placeholder prompt is replaced by the protocol renderer, view
  stack, result list, and action panel.
- Startup path: the registry must be populated and the store opened without
  breaking M0's 80ms activation budget. Indexing runs in the background, not on
  the activation path.
- A new on-disk database in the application data directory, and a cached icon
  store.
