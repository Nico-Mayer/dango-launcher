## Why

Audit finding A8-06: required and command-level preferences exist only on
paper. No built-in declares one, nothing reads a command-level value, the
config schema leaves them out, and the "prompts for the value" rule in
`extension-model` has no implementation. Dead contract surface drifts and
misleads the next reader.

## What Changes

- Replace the extension-model "Typed preferences" requirement with "Extension
  preferences": extension-level, optional preferences only. The "Required
  preference is missing" scenario goes.
- **BREAKING** (manifest): a preference declaration loses its command scope
  and its `required` flag.
- **BREAKING** (config file): a command's `preferences` field goes away. No
  built-in reads it; AI commands use their own typed fields (`provider`,
  `model`, `thinking`, `output`).
- Remove the unused helpers that only served the removed parts (the
  effective-value and unsatisfied checks).

## Non-goals

- No change to how extension-level preferences are read, defaulted, checked
  against a value set, or written back to the config file.
- No change to the config write path (A8-05 was dropped).
- No prompt-for-missing-value flow, now or as a stub.
- No manifest serde or `CommandOutcome` cleanup (A8-07, A8-08).

## Platforms

Platform-neutral. Windows and macOS are covered equally; no platform code is
touched.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `extension-model`: "Typed preferences" is removed and replaced by "Extension
  preferences" (extension-level, optional values). OpenSpec cannot drop a
  scenario through MODIFIED, so this is a remove plus add.
- `local-store`: "Preference values are stored and read back" loses its command
  scope and the "Extension and command scopes are separate" scenario, again as
  a remove plus add.

## Impact

- `src/extension/manifest.rs`, `src/extension/preferences.rs`: declaration
  fields, helpers, and tests.
- `src/config/mod.rs`, `src/config/store.rs`, `src/config/schema.rs`: the
  per-command `preferences` field and the command scope of the store.
- Built-in preference declarations in `clipboard`, `snippets`, and `ai`.
- `docs/config.md` (generated): the command `preferences` rows disappear.
- `configuration` and `config-schema` specs need no change: both already speak
  only of each extension's preferences.
