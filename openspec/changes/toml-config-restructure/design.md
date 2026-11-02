# Design

## Context

See proposal.md for why. Current shape, in `src/config/`:

- `Config` is one serde struct parsed with `serde_json`. Built-in settings
  live in `extensions: BTreeMap<String, Extension>`, where one `Extension`
  struct serves every extension. It holds `enabled`, `commands`,
  `preferences: BTreeMap<String, Value>`, plus `apps` (read only by
  applications) and `providers` (read only by ai).
- One `CommandSettings` struct serves every extension's commands, AI-only
  fields included. `schema.rs` strips those fields from other extensions'
  command schemas (`AI_ONLY`).
- Preferences are declared in each manifest (`PreferenceDecl` with a
  `PreferenceKind` of string, number, or boolean) and read through
  `Preferences` and `PreferenceStore`, which hand values back as strings and
  parse them per call. `schema.rs` turns those declarations into the
  `preferences` part of the schema.
- Core reads `enabled`, `command_enabled`, and `alias` by extension id;
  `hotkeys.rs` walks `extensions` for command and app hotkeys.
- `toml` 1.1.6 is already in `Cargo.lock` as a transitive dependency.

## Goals / Non-Goals

**Goals:**

- Each setting's type, default, description, and allowed values written once,
  on a Rust struct field, and the schema and `docs/config.md` derived from it.
- Core keeps reading enabled state, aliases, and hotkeys by extension id,
  without knowing which extension has which settings.
- Parse errors point at a line in `config.toml`.

**Non-Goals:**

- A settings model for extensions that are not built in. `extensions` gets
  the shared table shape and nothing more.
- Deny unknown keys at runtime. Forward compatibility stays (see the
  configuration spec); the schema is where unknown keys are flagged.

## Decisions

### 1. Parse with the `toml` crate, keep JSON Schema

Swap `serde_json::from_str` for `toml::from_str` in `Config::parse`. The
schema stays JSON Schema from schemars, written as `config.schema.json`, and
the created file's first line is `#:schema ./config.schema.json`. Taplo (Even
Better TOML) and Tombi both read that directive.

Rejected:

- `toml_edit`: preserves formatting for writing back. Dango never writes an
  existing file, so plain `toml` with serde is enough.
- A TOML-specific schema format: no editor reads one; both TOML language
  servers take JSON Schema.
- Keeping JSON with comments (JSONC/JSON5): fixes comments but not the deep,
  quoted paths or the multi-line prompts.

### 2. One typed struct per built-in table, as named fields on `Config`

`Config` gets one field per built-in, named by the id without `dango.`
(`clipboard`, `ai`, `applications`, `quicklinks`, ...), plus
`extensions: BTreeMap<String, Table>` for extensions that are not built in.

- Extensions with no settings of their own share
  `Table { enabled: Option<bool>, commands: BTreeMap<String, CommandSettings> }`.
- Extensions with settings get their own struct that writes `enabled` and
  `commands` out as fields beside their settings:
  `ClipboardTable { enabled, commands, entries: u32, entry_megabytes: u32,
  total_megabytes: u32, excluded_applications: Vec<String>, primary_action:
  PrimaryAction }`.
- `ai` gets `AiTable { enabled, commands: BTreeMap<String, AiCommand>,
  providers, provider: Option<String>, thinking: Thinking }`. `AiCommand`
  carries `alias`, `hotkey`, `enabled` plus the AI fields, so `CommandSettings`
  loses `title`, `prompt`, `provider`, `model`, `thinking`, `output`, and
  `schema.rs` loses `AI_ONLY`.
- `applications` gets `hotkeys: BTreeMap<String, AppSettings>` (was `apps`).
- `thinking`, `output`, and `primary-action` are serde enums, not strings.
  `Thinking::parse`, `Thinking::NAMES`, and `Output::parse` go.
- Defaults live in `Default` impls and `#[serde(default)]`, so a field that
  has a default is not an `Option`. Doc comments are the descriptions.
- Every struct is `#[serde(rename_all = "kebab-case")]`; `ProviderSettings`
  drops `camelCase`.

Rejected:

- Keep manifest preference declarations and add a `list` kind: rebuilds what
  serde and schemars already do, with a second type system parsing strings.
  Only a handful of preferences exist, all in built-ins.
- `#[serde(flatten)]` of a shared `{ enabled, commands }` part into each
  table: flatten buffers through serde's internal content type, which loses
  the TOML error spans and makes number handling unreliable. Writing two
  fields out per struct is cheaper than worse error messages.
- A generic `Table<S>` with the settings in a `settings` sub-table: puts the
  `preferences` level back that the restructure removes.

### 3. Core reads tables by id through one small trait

A trait, implemented by each table struct:

```rust
trait ExtensionTable {
    fn enabled(&self) -> Option<bool>;
    fn command(&self, id: &str) -> Option<CommandBinding<'_>>;
    fn commands(&self) -> Box<dyn Iterator<Item = (&str, CommandBinding<'_>)> + '_>;
}
```

`CommandBinding` borrows `enabled`, `alias`, and `hotkey`.
`Config::table(extension_id) -> Option<&dyn ExtensionTable>` is one `match`
on the built-in ids, falling back to `extensions.get(id)`. `enabled`,
`command_enabled`, `alias`, and the hotkey walk in `hotkeys.rs` go through it.
The table name comes from stripping `dango.` off the manifest id, in one
function used by the match, the schema, and the reserved-name test.

Rejected: a `BTreeMap<String, Box<dyn ...>>` built at parse time. Needs
custom deserialization for no gain over a match of ten arms.

### 4. Extensions read their settings from the shared config

The `Preferences` reader, `PreferenceStore`, `PreferenceDecl`,
`PreferenceKind`, and `Manifest::preferences` are removed. Where an extension
was handed a `Preferences` reader, it is handed the shared
`Arc<RwLock<Config>>` (or a closure over it) and reads its own table, e.g.
`config.read().unwrap().clipboard.entries`. A reload swaps the config behind
the lock, so reads stay live the way preference reads were.

The clipboard's and selection's comma splitting goes; both read
`Vec<String>` and compare ignoring case.

Manifest contract: the manifest loses `preferences`, and "contributes
nothing" no longer counts preferences. `SUPPORTED_MANIFEST_VERSION` stays 1:
every manifest today is compiled in, and alpha keeps no compat path.

### 5. A bad value rejects the file

A wrong type or an unknown enum value fails `toml::from_str`, so the existing
"keep the last good config, surface the error" path handles it with no new
code. The tray and log show the `toml` error text, which names the line and
the key. Unknown keys still deserialize fine because no struct uses
`deny_unknown_fields`.

Rejected: per-field leniency (`serde_with::DefaultOnError`). It keeps the old
"wrong type reads as default" behaviour but hides the mistake, and adds a
dependency and an annotation on every field.

### 6. Schema generation

`schemars::schema_for!(Config)` now produces the extension tables directly,
with types, defaults, enums, and descriptions. What `schema.rs` still does by
hand:

- Close each built-in table's `commands` to its manifest's shipped command
  ids. `ai.commands`, `ai.providers`, `applications.hotkeys`, and
  `extensions` stay open maps.
- The existing passes: strip `null`, close objects, tidy descriptions,
  the hotkey pattern.

The preference-to-schema code and `AI_ONLY` are deleted. `docs/config.md` is
regenerated from the same schema; its generator keeps working on the schema
value, with section paths now starting at the table name.

### 7. Hyperkey uses `enabled`

`Config::hyperkey` becomes `Hyperkey { enabled: bool (default false), shift,
tap }`, not `Option<Hyperkey>`. Code that checked `is_some()` checks
`enabled`.

### 8. No interface copy changes

Launcher and tray strings say "config file" and never the file name, so none
change. The parse error shown is the `toml` crate's text, as the `serde_json`
text was before. README's configuration section and the header of
`docs/config.md` name the file and are updated.

## Risks / Trade-offs

- [Cutover turns autostart off on the author's machine] → Write
  `config.toml` from the current `config.json` before installing the release
  with this change. A task covers the conversion of the author's file.
- [Taplo or Tombi may not handle schemars' draft 2020-12 output] → schemars
  can emit draft-07 (`SchemaSettings::draft07()`). Check once in the editor
  while applying; switch the settings if completion or checks are missing.
- [A typo anywhere now rejects the whole file] → Intended: the error names
  the line, and the last good config keeps running. At startup it means
  defaults until fixed, the same as a syntax error today.
- [TOML 1.1 syntax in examples] → `toml` 1.1 accepts multi-line inline
  tables, but editors on TOML 1.0 may flag them. Docs and the created file
  use only TOML 1.0 syntax.
- [Untagged `Hotkey` and `AppName` give vague errors] → Unchanged from today;
  out of scope.
- [Conflicts with `add-agent-cli-providers`] → That change edits
  `ProviderSettings` and the ai-providers spec. Archive it first.

## Migration Plan

No data migration. Steps for the author:

1. Archive `add-agent-cli-providers`.
2. Apply this change.
3. Before installing the release, rewrite `~/.config/dango/config.json` as
   `config.toml` (table per extension, `enabled = true` in `[hyperkey]`,
   arrays for exclusion lists, renamed keys). Delete `config.json` once the
   new file loads.

Rollback: reinstall the previous release; `config.json` is still read there
as long as it was not deleted.
