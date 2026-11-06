# Proposal

## Why

Every feature today is a native built-in, so a small integration (list my pull
requests, run a CLI, call a webhook) means Rust code in the core. Integrations
with services such as GitLab or Jira change with those services' APIs, and
should not need a Dango release each time. Extensions written in JavaScript and
loaded from a folder let such features live outside the core and be shared as a
folder (#33).

## What Changes

- Load extensions that are not built in from `extensions/<id>/` in the config
  directory. Each is a folder with `manifest.json` and an ES module `main.js`.
- Run each extension's JavaScript in an embedded QuickJS runtime that exists
  only while it has work: a command running, a view open, or root items being
  built. No extension costs memory while idle.
- Declared commands appear in root search from the manifest alone, with
  aliases, hotkeys, and per-command `enabled` like built-ins.
- Cached root items: an extension can return rows for root search once, on
  load and optionally on an interval. Dango keeps and filters them natively.
- Actions in an extension's views run JavaScript and can push a new view,
  replace the current one, show a ticker, go back, or finish.
- Broad, trusted capabilities: run programs, read and write files, make
  network requests, timers, open URLs and files, copy, paste, per-extension
  storage, and preferences from the config file.
- Secrets: an extension reads its own keys from `[extensions.<id>]` in
  `auth.toml`, so tokens for services like GitLab or Jira stay out of
  `config.toml`.
- Live search: a list or grid can ask to receive the query and search itself.
  The view protocol stays at version 2; the new field is optional.
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
- npm installs inside Dango, and Node built-ins (`require`, `Buffer`,
  `process`). Authors bundle pure-JS dependencies into `main.js` themselves.
- OAuth login flows. Services are reached with tokens from `auth.toml`.
- Languages other than JavaScript, and a Node.js or Bun runtime.
- Rewriting built-ins as JavaScript extensions.

## Platforms

Windows and macOS, both fully. The feature is platform-neutral apart from
running programs and the folder watch, which already exist on both.

## Capabilities

### New Capabilities

- `js-extensions`: loading, running, and reloading JavaScript extensions from
  the extensions folder; their manifest file, runtime lifetime, capabilities,
  secrets, cached root items, action handling, preferences, storage, failure
  isolation, and the generated type definitions.

### Modified Capabilities

- `extension-model`: services stay a built-in-only contribution, and a root
  items provider of a JavaScript extension is a cached one.
- `view-protocol`: a list or grid can hand searching to its command, which
  then receives the query as the user types; the item limit follows the
  command's order there.
- `configuration`: `extensions.<id>` also holds a JavaScript extension's own
  preferences.
- `config-files`: the `extensions` folder in the config directory holds
  extension code, not user data files.
- `ai-providers`: `auth.toml` also holds extension secrets under
  `[extensions.<id>]`.

## Impact

- New `src/js/` module for the JavaScript host beside `src/extension/`.
- `ExtensionHost` gains `remove` for deleted extension folders.
- `Manifest` gets a JSON reader in `src/js/`; the manifest version stays 1.
- `src/protocol.rs` and the `src/ui/` state machine: the `filtering` field and
  live search.
- Config schema: `extensions.<id>` accepts extension-defined keys.
- `auth.toml` reader: a lookup for extension secrets.
- Local database: one machine-local table for extension storage.
- New dependencies: `rquickjs` (async) and `rquickjs-extra` (timers, url);
  `ts-rs` as a dev-dependency for the types file.
- `AGENTS.md`: replace "sandboxed host" with the trusted JavaScript host.
