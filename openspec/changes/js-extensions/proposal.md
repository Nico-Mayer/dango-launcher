# Proposal

## Why

Every feature today is a native built-in, so a small integration (list my pull
requests, run a CLI, call a webhook) means Rust code in the core. Extensions
written in JavaScript and loaded from a folder let such features live outside
the core and be shared as a folder (#33).

## What Changes

- Load extensions that are not built in from `extensions/<id>/` in the config
  directory. Each is a folder with `manifest.json` and an ES module `main.js`.
- Run each extension's JavaScript in its own embedded QuickJS runtime. A
  runtime exists only while it has work: a command running, a view open, or
  root items being built. No extension costs memory while idle.
- Declared commands appear in root search from the manifest alone, with
  aliases, hotkeys, and per-command `enabled` like built-ins.
- Cached root items: an extension can return rows for root search once, on
  load and optionally on an interval. Dango keeps and filters them natively.
- Actions in an extension's views run JavaScript and can push a new view,
  replace the current one, show a ticker, or finish.
- Broad, trusted capabilities: run programs, read and write files, make
  network requests, timers, open URLs and files, copy, per-extension storage,
  and preferences from the config file.
- **BREAKING** View protocol version 3: a list or grid can ask to receive the
  query and filter itself (live search). Version 2 trees are rejected.
- Dango writes a generated `dango.d.ts` into the extensions folder for editor
  completion.
- Editing an extension's files reloads it without a restart. A broken
  extension is reported and never affects others.

## Non-goals

- Permissions, sandboxing, signing, or an install prompt. Extensions are fully
  trusted. Sharing is copying a folder.
- An install command, store, index, or update mechanism.
- Background services for JavaScript extensions.
- Live root items (JavaScript called on every root keystroke).
- npm installs inside Dango. Authors bundle dependencies into `main.js`
  themselves.
- Secrets for extensions in `auth.toml`. An extension that needs a token uses
  a CLI that holds it, or its own storage.
- Languages other than JavaScript, and a Node.js or Bun runtime.
- Rewriting built-ins as JavaScript extensions.

## Platforms

Windows and macOS, both fully. The feature is platform-neutral apart from
running programs and the folder watch, which already exist on both.

## Capabilities

### New Capabilities

- `js-extensions`: loading, running, and reloading JavaScript extensions from
  the extensions folder; their manifest file, runtime lifetime, capabilities,
  cached root items, action handling, preferences, storage, failure isolation,
  and the generated type definitions.

### Modified Capabilities

- `extension-model`: services stay a built-in-only contribution, and a root
  items provider of a JavaScript extension is a cached one.
- `view-protocol`: version 3; a list or grid can hand filtering to its command,
  which then receives the query as the user types.
- `configuration`: `extensions.<id>` also holds a JavaScript extension's own
  preferences.
- `config-files`: the `extensions` folder in the config directory holds
  extension code, not user data files.

## Impact

- New module for the JavaScript host beside `src/extension/`; a second
  `registry::Host` variant.
- `Manifest` gains a JSON reader; the manifest version stays 1.
- `src/protocol.rs` and `src/ui/` state machine: version bump and live search.
- Config schema: `extensions.<id>` accepts extension-defined keys.
- Local database: one machine-local table for extension storage.
- New dependencies: `rquickjs` (async), Node-compatible modules for it, and
  `ts-rs` for the types file. Details and fallbacks in design.
- `AGENTS.md`: replace "sandboxed host" with the trusted JavaScript host.
