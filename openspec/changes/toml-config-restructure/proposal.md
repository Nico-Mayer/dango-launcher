# Proposal

## Why

The config file has grown, and it is now hard to read and write. Paths are deep
and need quoting (`extensions."dango.window-management".commands.left-half`),
preferences sit in a separate `preferences` object while other settings of the
same extension sit beside it, casing is mixed (`baseUrl`), lists are comma
separated strings, and multi-line AI prompts are one escaped JSON line. There
is also no written rule for where a new setting goes, so each one is placed by
taste.

## What Changes

- **BREAKING**: the config file is `config.toml`, in TOML. `config.json` is no
  longer read. No converter and no warning: the author rewrites their file once.
- **BREAKING**: the editor finds the schema through a `#:schema
  ./config.schema.json` comment on the first line. The `$schema` key is gone.
  The schema itself stays JSON Schema, generated from the code.
- **BREAKING**: each built-in extension has its own top-level table, named by
  its id without `dango.`: `[clipboard]`, `[window-management]`, `[ai]`. The
  `extensions` table is reserved for extensions that are not built in.
- **BREAKING**: an extension's settings sit directly in its table, typed. The
  `preferences` object is gone. Lists are TOML arrays, so `excluded-applications`
  in `[clipboard]` and `[selection]` become arrays.
- **BREAKING**: a value of the wrong type or outside its allowed values makes
  the file invalid, like a syntax error: Dango keeps the last good config and
  says where the problem is. Before, such a value quietly read as its default.
- **BREAKING**: `enabled` is the one on/off key, in every table that can be
  turned off. `[hyperkey]` needs `enabled = true`; the table being present is
  no longer enough.
- **BREAKING**: renames. `applications.apps` becomes `applications.hotkeys`,
  `baseUrl` becomes `base-url`, `launcher.reset-after` becomes
  `launcher.reset-after-seconds`. Every key is kebab-case.
- Written rules for the file's layout, so the next setting has one obvious
  place: which tables exist, reserved keys, nesting depth, units in key names,
  lists, per-platform values, and what an absent key means.
- **BREAKING** (extension manifest contract): built-in manifests no longer
  declare preferences. A built-in's settings are a typed struct that also
  drives its part of the schema and the settings reference.

## Non-goals

- A settings UI.
- Settings for extensions that are not built in. The `extensions` table is only
  reserved. Whether such an extension's own settings live in `config.toml` or in
  a file of its own is decided when the extension host is designed.
- Reading `config.json`, converting it, or warning about it.
- Moving credentials. `auth.json` stays as it is.
- Changing what any setting does. Only its name, place, or type changes.
- Writing to an existing config file. Dango still only creates a missing one.

## Platforms

Windows and macOS alike. Parsing, the schema, and the settings structs are
shared code with no platform branches, so there is nothing to check by hand on
either platform.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `configuration`: TOML file `config.toml`; top-level tables per built-in
  extension; typed settings in the table; wrong-type values invalidate the file;
  the layout rules.
- `config-schema`: schema linked by a `#:schema` comment instead of a `$schema`
  key; settings described from typed structs, not preference declarations; a
  created file starts with the comment.
- `extension-model`: built-in extensions no longer declare preferences;
  command config path changes to `<table>.commands.<id>`.
- `command-hotkeys`: command and application hotkey paths.
- `clipboard-history`: settings live in `[clipboard]`; `excluded-applications`
  is a list; an unknown `primary-action` invalidates the file.
- `keyword-expansion`: path of the shared exclusion list.
- `selection-and-paste`: `selection.excluded-applications` is a list.
- `hyperkey`: turned on by `enabled = true`, not by the table being present.
- `launcher-shell`: `launcher.reset-after` becomes `launcher.reset-after-seconds`.
- `system-commands`: what Open Config File creates, and the example hotkey path.
- `quicklinks`: `favicons` is a setting at `quicklinks.favicons`.
- `ai-providers`: keys are kept out of `config.toml` (name only).
- `interface-copy`: "config file" names `config.toml`.

## Impact

- Code: `src/config/` (parser, structs, schema, reference, created template,
  location), `src/extension/preferences.rs` and `PreferenceStore` (removed),
  `src/extension/manifest.rs` (preference declarations removed), every built-in
  that reads preferences (clipboard, snippets, quicklinks, ai), `src/app/`
  reload and hotkey lookups, every test that builds a config from JSON text.
- Dependencies: add the `toml` crate. `serde_json` stays for the schema.
- Docs: `docs/config.md` regenerated.
- Author's machine: `config.toml` must exist before installing the release with
  this change, or autostart turns off and the hotkey resets to the default.
- Order: `add-agent-cli-providers` changes `ai-providers` and the provider
  struct, so it is archived before this change is applied.
