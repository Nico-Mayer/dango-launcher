## Context

See proposal.md for the motivation. What shapes the approach:

- The config is a tree of serde structs in `src-tauri/src/config/mod.rs`
  (`Config`, `Launcher`, `Hyperkey`, `Selection`, `Extension`,
  `CommandSettings`, `AppSettings`, `ProviderSettings`, `Hotkey`,
  `AppName`), each with a flattened `extra` map that preserves unknown keys
  and doc comments on most fields. `Hotkey` and `AppName` are untagged
  string-or-object enums. `ProviderKind` carries an `Unknown(String)`
  variant; `thinking` and `output` are plain strings on `CommandSettings`
  although the AI extension parses them into its own enums.
- Every built-in extension has a `manifest()` function returning its id,
  commands, and `PreferenceDecl`s (key, kind, default, required). The AI
  extension builds its manifest from a static `SHIPPED` list plus the config;
  the snippets and quicklinks extensions share one manifest parameterised by
  kind. Declarations have no description and no allowed-values list. The AI
  extension declares no preferences at all and reads `provider` and
  `thinking` from the config directly, so a manifest walk would miss them
  until they are declared.
- The hotkey parser in `config/hotkey.rs` matches modifier and key tokens
  inline in two `match` expressions; nothing else can enumerate them.
- `docs/config.schema.json` is hand-written and already lacks
  `launcher.autostart`. `docs/config.example.jsonc` is hand-written prose.
  Two tests in `config/mod.rs` validate the example against the schema with
  the `jsonschema` dev dependency.
- `config::watch` reacts only to events whose file name is `config.json`, so
  a sibling file in the directory does not trigger a reload.
- `schemars` 0.8 is already compiled as a build dependency of `tauri-build`;
  1.x is not on this target, so it is a new direct dependency.
- The project is in alpha with no users: shapes may change without migration.

## Goals / Non-Goals

**Goals:**

- One model, produced by the app crate, that yields the schema file, the
  settings reference, and nothing else maintained by hand.
- The generator is ordinary Rust with unit tests; no build script, no
  separate binary, no code generation step in CI beyond `cargo test`.
- Descriptions are written once, next to the thing they describe.

**Non-Goals:**

- Making the loader validate against the schema. The schema is editor
  feedback and documentation; the loader stays as it is.
- A general plugin-schema facility. Only built-in manifests are walked.

## Decisions

### `schemars` 1.x derives the static half; a manifest walk adds the dynamic half

The config structs get `#[derive(JsonSchema)]`. Doc comments become
descriptions, `Option` fields become optional properties, untagged enums
become `anyOf`, and unit enums become `enum` lists. That covers every
setting that is not keyed by an extension id.

The dynamic half is a function in a new `config::schema` module that takes
the built-in manifests and builds `extensions.properties.<id>` for each one:
`enabled`, `preferences` with one property per `PreferenceDecl` typed from
`PreferenceKind` with `default`, `description`, and `enum` from `values`,
and `commands` with one property per shipped command id. The per-command
object is the generic command settings for every extension except
`dango.ai`, whose commands also take `title`, `prompt`, `provider`, `model`,
`thinking`, and `output`, and whose `commands` map stays open for
user-defined ids. `dango.applications` gets `apps`; `dango.ai` gets
`providers`.

To obtain the manifests the generator takes a `&[Manifest]` so it has no
dependency on the host; `lib.rs` passes the same manifests it registers,
and the tests pass the built-in `manifest()` functions directly. The AI
manifest is built from its shipped list with an empty config.

Alternatives considered:

- Keep hand-writing the schema and add a test that every struct field appears
  in it. Catches omissions but not wrong types, descriptions, or the
  per-extension keys the hand schema cannot know. Rejected.
- Write the whole schema in Rust with `serde_json::json!`. One copy of every
  field name beside the struct that already has it. Rejected.
- `schemars` 0.8 to avoid a new version in the tree. It is a build
  dependency only, so it is not in the app binary anyway, and 1.x is the
  maintained line with the cleaner `Schema` type. Rejected.

### A post-pass closes every object the schema names

`schemars` emits `additionalProperties: true` for the flattened `extra` maps.
After generation, one pass walks the schema and sets `additionalProperties`
to `false` on every object that has `properties`, then reopens exactly three
maps: `apps` under `dango.applications`, `providers` under `dango.ai`, and
`commands` under `dango.ai`, each with the matching item schema as
`additionalProperties`. The `extensions` object itself is closed to the
built-in ids.

Alternative: `#[schemars(deny_unknown_fields)]`-style attributes per struct.
They do not compose with the per-extension objects the walk constructs, and
one pass is easier to read than attributes on nine structs. Rejected.

### Tighten three config types so the schema can state their values

Under the alpha stance:

- `ProviderKind` drops `Unknown(String)` and becomes a plain
  `#[serde(rename_all = "lowercase")]` enum. A kind Dango does not know is a
  parse error of that provider entry, surfaced like any other config error.
- `CommandSettings::thinking` becomes `Option<Thinking>` and
  `CommandSettings::output` becomes `Option<Output>`, the enums the AI
  extension already defines, moved into `config` so the struct owns its
  types. The AI extension stops parsing strings.
- `Config` gains `schema: Option<String>` serialised as `$schema`, declared
  first so `to_json` writes it first. `ensure_file` sets it to
  `./config.schema.json` on the file it creates.

### Hotkey tokens become data the parser and the schema share

`hotkey.rs` gains `pub const MODIFIER_TOKENS: &[&str]` and
`pub const KEY_TOKENS: &[&str]`, and its two `match` expressions become
lookups against those tables, so a token cannot be accepted by one and
unknown to the other. The schema's hotkey string form gets
`pattern`: one or more modifier tokens joined by `+`, then one key token, a
single lowercase letter or digit, or `f1` to `f12`. `hyperkey.tap` becomes
`enum` over `KEY_TOKENS` plus the single characters. A test round-trips every
token through both the parser and the pattern.

The pattern is lowercase only. The parser lowercases its input, so
`Hyper+Space` still works at runtime and merely gets a squiggle; a
case-insensitive ECMA pattern would need a character class per letter and
is not worth it.

### Written beside the config at startup, compared before writing

`lib.rs` calls `config::schema::write(&config_dir, &manifests)` after the
manifests are registered and before the watcher starts. It serialises the
schema with `to_string_pretty`, reads the existing file, and writes only when
the bytes differ. A failure logs `config: could not write
config.schema.json: <error>` and returns; no tray notice, because nothing the
user relies on is affected. The watcher ignores the file by name, which the
existing `watch` test covers; a new test asserts the schema write does not
set the own-write signal or touch `config.json`.

Alternatives: embed a pre-generated schema with `include_str!` and copy it
(a second artifact to keep current; rejected); write only when the version
changes (the content comparison is one read and is always right; rejected).

### `docs/config.md` is rendered from the schema and checked by a test

A renderer in `config::schema` walks the final schema and emits Markdown:
one section per object, one table row per property with path, type, default,
allowed values, and description, in schema order. The test
`the_settings_reference_is_current` compares `docs/config.md` with the
rendered text. With `DANGO_UPDATE_DOCS=1` in the environment it writes the
file and passes; otherwise a difference fails with:

```
docs/config.md is out of date. Run:
  env DANGO_UPDATE_DOCS=1 cargo test the_settings_reference_is_current
```

Alternative: an `hk` fix step that rewrites the file on commit. CI runs tests,
not hooks, and a hook that edits files during a commit surprises. Rejected.
A separate `xtask` binary was rejected as more plumbing for one function the
test already reaches.

The two existing docs tests and both hand-written docs files are deleted. The
`jsonschema` dev dependency stays, used by new tests that validate sample
configs against the generated schema.

### Preference declarations carry their documentation

`PreferenceDecl` gains `description: String` and `values: Vec<String>`.
`Manifest::validate` rejects a preference with an empty description, so a new
preference cannot ship undocumented. The preference reader returns the
default for a stored string outside a non-empty `values` list, the same way
it does for a value of the wrong type. The prose from
`docs/config.example.jsonc` moves into these fields and into the struct doc
comments, trimmed to the interface-copy rules.

### User-facing text

Checked against `openspec/specs/interface-copy/spec.md`: sentence case,
plain words, sentences under 20 words, nothing internal.

- Log, schema write failed: `config: could not write config.schema.json: <error>`
- Test failure: the two lines quoted above
- Manifest validation error: `extension '<id>' declares preference '<key>' without a description`
- `docs/config.md` header: `# Dango configuration` followed by `Every
  setting in config.json. Generated from the code; do not edit by hand.`
- Preference descriptions, moved from the example file (final text set in
  the task, each one sentence, no identifiers):
  - clipboard `entries`: `How many entries the history keeps.`
  - clipboard `entry-megabytes`: `The largest single entry kept, in megabytes.`
  - clipboard `total-megabytes`: `The most the whole history may use, in megabytes.`
  - clipboard `excluded-applications`: `Applications whose copies are never recorded, comma separated.`
  - clipboard `primary-action`: `What Enter does to an entry: paste it into the previous app, or only copy it.`
  - quicklinks `favicons`: `Show each quicklink with its site's icon. Off stops Dango fetching from those sites.`
  - ai `provider`: `The provider a command uses when it names none.`
  - ai `thinking`: `How hard the model thinks when a command does not say.`

README, configuration section, replacing the sentence that names the example
file:

> Every setting is listed in `docs/config.md`. Dango writes
> `config.schema.json` beside the config file; put
> `"$schema": "./config.schema.json"` at the top of yours and your editor
> will complete and check it.

### Found while implementing

- `jsonschema` 0.26 has no meta-schema module, so the structural check is
  `validator_for`, which refuses to compile a malformed schema. Good enough;
  the dev dependency stays where it is.
- A `thinking`, `output`, or provider `kind` outside its enum fails the
  whole file, not the one entry, because serde rejects the tree. That is how
  every other wrong type already behaves and the error names the value.
- The per-platform hotkey object is inlined in the Hotkey def rather than
  patterned on `PerPlatform`, which application names share.
- Command-scoped preference declarations are not rendered per command. No
  built-in declares one; the generator ignores them until one exists.
- The AI extension declares `provider` and `thinking` so the walk sees them,
  but still reads them from the config directly, through the typed parse.
- A tidy pass flattens doc-comment line breaks into one line and turns
  whole-number float defaults into integers, for the hover and the table.
- The reference omits an object with nothing to set, such as `preferences`
  of an extension that declares none, instead of printing an empty table.

## Risks / Trade-offs

- [Relative `$schema` may not resolve in some editor] → VS Code's JSON
  language server, which Helix and Zed also use, resolves `$schema` against
  the document. Verified in the platform tasks rather than assumed; if an
  editor needs an absolute path, the README gets a second line, not the
  generator.
- [`schemars` output shape changes on upgrade and the docs test fails] →
  The failure names the regenerate command; the diff is reviewed like any
  other. Pin the minor version.
- [A closed `extensions` map rejects a future third-party extension's
  settings] → Intended for now; the alpha stance says reopen it when the host
  exists.
- [Dropping `ProviderKind::Unknown` makes an unknown kind a load error] →
  Surfaced through the existing config error path; alpha stance.
- [The schema file appears in the user's dotfiles directory] → It is
  generated and should be ignored by their git; the README says so. Dango
  does not manage their ignore file.
- [Two Dango versions on two machines write different schemas to a shared
  dotfiles directory] → Each machine's file matches its own build after
  startup; with the file ignored by git, nothing is shared.

## Migration Plan

No migration: alpha stance. A config with an unknown provider kind or a
`thinking` value outside the enum fails to parse that entry; the rest of the
file loads as before. Rollback is reverting the change; the schema file in
the config directory is harmless if left behind.
