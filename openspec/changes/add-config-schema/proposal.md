## Why

The settings and keymap live in `config.json`, and the facts about them are
kept by hand in three places: the Rust structs, `docs/config.schema.json`,
and `docs/config.example.jsonc`. They have already drifted (`launcher.autostart`
is missing from the schema), the schema cannot name an extension's preference
keys or command ids, and nothing points an editor at it. As the file grows,
the only way to keep one truth is to generate it from the code.

## What Changes

- A schema generator in the app, built from the config structs and the
  built-in manifests, so every setting, preference key, shipped command id,
  default, and description comes from the code that reads it.
- Dango writes `config.schema.json` beside `config.json` at startup, and the
  file it creates through Open Config File starts with
  `"$schema": "./config.schema.json"`, so an editor's JSON language server
  completes and checks the file against the running version.
- The schema is strict: every known object rejects unknown keys. The loader
  keeps preserving them at runtime, so a stray key still cannot break the app.
- Hotkeys are checked by a pattern built from the parser's own modifier and
  key tables, and `hyperkey.tap` becomes an enum of the same keys.
- Preference declarations gain a description and, for strings, allowed
  values, so one sentence next to the default serves the schema and the docs.
- **BREAKING** (free under the alpha stance): `ProviderKind` loses its
  unknown variant, and a command's `thinking` and `output` become typed enums
  in the config, so the schema can state their values.
- `docs/config.md` is generated from the same model: every path, its type,
  default, and description. A test fails when it is stale and names the
  command that regenerates it.
- `docs/config.schema.json` and `docs/config.example.jsonc` are deleted along
  with their tests. The README points at the generated docs and the one-line
  `$schema` key.
- One new dependency: `schemars`.

Milestone: M7 - polish. Platforms: Windows and macOS, both covered; the only
platform difference is the config directory, which is already shared code.

## Non-goals

- Adding `$schema` to an existing `config.json` at startup. The file lives in
  dotfiles, and a startup write would mean one git diff per machine. The
  created file has it; the README says to add it to an older file.
- Validating `config.json` against the schema when loading it. The loader
  stays lenient so a bad key never takes the app down.
- Schemas for `auth.json`, `snippets.json`, or `quicklinks.json`.
- Describing third-party extensions. The `extensions` map is closed to the
  built-in ids and reopens when a third-party host exists.
- A hosted copy of the schema or any `$id` URL that resolves.
- Commands carrying a description. Their title and the static settings they
  take are enough for completion today.

## Capabilities

### New Capabilities

- `config-schema`: the generated JSON Schema for the config file, where it is
  written, what it covers and how strictly, and the generated settings
  reference that is kept from going stale.

### Modified Capabilities

- `configuration`: the config directory now holds a schema file that Dango
  creates unasked; the "no file means defaults" scenario is narrowed to the
  config file itself.
- `extension-model`: typed preferences carry a description and, for strings,
  an allowed-values list.
- `system-commands`: the config file created by Open Config File starts with
  the `$schema` key.

## Impact

- `src-tauri/Cargo.toml`: adds `schemars`.
- `src-tauri/src/config/`: a new `schema` module; `mod.rs` gains the
  `$schema` field, typed `thinking` and `output`, and a plain `ProviderKind`;
  `hotkey.rs` exposes its modifier and key tables.
- `src-tauri/src/extension/manifest.rs`: `PreferenceDecl` gains `description`
  and `values`; every built-in manifest fills them.
- `src-tauri/src/lib.rs`: one startup call that writes the schema file.
- `src-tauri/src/extensions/system/`: the created file gains `$schema`.
- `docs/`: `config.md` generated; `config.schema.json` and
  `config.example.jsonc` removed; two tests in `config/mod.rs` removed.
- `README.md`: configuration section.
- No manifest or protocol version change: the manifest gains fields, which
  the alpha stance makes free.
