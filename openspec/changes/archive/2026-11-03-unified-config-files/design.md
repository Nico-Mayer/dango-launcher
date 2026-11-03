# Design

## Context

- `src/config/schema.rs` builds one schema for `config.toml` with `schemars`,
  then post-processes it (strip `null`, close objects, hotkey pattern, inline
  extension tables). It writes `config.schema.json` beside the config and
  renders `docs/config.md`.
- `src/config/store.rs` creates a missing `config.toml` with its `#:schema`
  header and records the text as its own write.
- `src/extensions/snippets/store.rs` holds records as `serde_json::Map` so
  unknown fields survive, fills missing ids with `name:<name>`, and rewrites
  the whole file through a temp file and rename, recording its own write.
- `src/extensions/ai/auth.rs` reads `auth.json` on each use, reports a parse
  error by line only, and restricts permissions on macOS.
- `src/config/watch.rs` already serves the config and both record files: it
  watches the directory without recursion, filters by file name, and ignores
  the app's own write.
- File paths come from three places: `config_path` and `auth_path` in
  `src/config/location.rs`, and `Kind::file_name` in the record store.
- A bad config is logged as `config reload failed: the configuration file is
  not valid: <toml message>`; a bad record file as `snippets.json: <message>`.

See proposal.md for why. The rules are in `specs/config-files`.

## Goals / Non-Goals

**Goals:**

- One list in `src/config/` names the user files. Path, `#:schema` header,
  schema, and reference section all come from it.
- One write helper for every Dango write to a user file.
- One log format for a bad file: `<file>: <message with line>`.
- Record writes keep the user's text.
- Secrets never show up in an error.

**Non-Goals:**

- A shared generic loader. Config, records, and auth load differently (typed
  and watched, edited and watched, read on use). One loader type over all
  three adds indirection without removing code.
- Changing how config errors reach the tray. Only the log line changes.

## Decisions

### One list of user files

`src/config/files.rs` (name open) holds the four files:

```
UserFile { name: "snippets", schema: fn() -> Value }
  .path()        config_dir()/snippets.toml
  .schema_path() config_dir()/.schemas/snippets.schema.json
  .header()      "#:schema ./.schemas/snippets.schema.json\n"
```

`config_path`, `auth_path`, and `Kind::file_name` read from it, and
`SCHEMA_REF` goes. `schema.rs` loops over the list for writing and for
`docs/config.md`, running the shared post-processing (`strip_null`,
`close_objects`, `tidy`) for each file:

```
config      schemars(Config) + existing hotkey and extension steps
snippets    schemars(SnippetEntry), top level open to ids, `version` allowed
quicklinks  schemars(QuicklinkEntry), `browser` enum from `Browser`
auth        object of string values, `version` allowed
```

The entry structs exist for the schema only. Their doc comments become the
field descriptions in the schema and the reference.

Alternative: a separate generator per file. The author ruled this out; it
repeats the post-processing and the write logic.

### One write helper

`write_user_file(path, text, own_write)` does what the record store does today:
create the folder, write a temp file, set `own_write`, rename, and clear
`own_write` when the rename fails. Config's `ensure_file` and the record store
both call it, so a created `config.toml` is written atomically too.

### `toml_edit` for record files

The record store parses with `toml_edit::Document::parse`. That keeps spans,
so `Item::span` gives the line of a field with the wrong type. It projects
each table into the existing `Record`. Projection checks types: a non-string
`name`, `template`, `url`, `keyword`, or `browser` makes the file invalid with
that line, the same way `toml` reports a wrong type in `config.toml`. Then
`into_mut` gives the `DocumentMut` the store keeps for edits.

Create, update, and remove change only the touched table, then write the
document back through the write helper. Comments, entry order, unknown keys,
and untouched entries survive because they are never re-serialised. A new
template with a newline needs no special code: `toml_writer` picks a
multi-line string for such text by default. A test checks the file text.

`toml_edit` 0.25 is in `Cargo.lock` today only as a build dependency through
Slint (`proc-macro-crate`). It becomes a runtime dependency, at the version
already locked.

Alternatives:
- `toml::to_string` of typed structs: loses comments and order on every write.
  That breaks "Each file has one owner".
- `toml::Table` with `preserve_order`: keeps order, still loses comments.
- Deserialising each entry from a `toml::Value` into a struct: the value has
  no span, so a wrong type could not name its line.
- `taplo` / `tombi` as a library: built for language servers, with their own
  syntax tree API for a job `toml_edit` already does.

### Ids from a slug crate

New ids come from the `slug` crate (`slugify`, built on `deunicode`): it
lowercases, folds `é` and `ü` to ASCII, and joins words with `-`. The store adds
what the spec asks for: an empty slug becomes `snippet` or `quicklink`,
`version` is skipped, and a taken id gets `-2`, `-3`, and so on. Hand-written
ids stay exactly as the user wrote them.

Alternatives: `heck` makes kebab-case but does not fold accents, so "Café"
would keep `é`. A hand-rolled ASCII filter would drop `ü` instead of turning
it into `u`. `slug` is not in the registry yet; check its API after adding it.

The `name:<name>` fallback and its tests go: in TOML every entry has a key.

### Schemas in `.schemas/`, `.schema.json` kept

The `#:schema` comment is resolved relative to the TOML file by Taplo and
Tombi, so `./.schemas/config.schema.json` works. The suffix stays so that
`config.json` never means two things, and a search for an old JSON file name
finds nothing. The watcher is not recursive and filters by file name, so
schema writes in the subfolder never look like user edits. No test is added
for that.

Alternatives: `schemas/` without a dot shows up in Finder; schemas beside the
files, as today, mix four generated files with the user's own.

### One log format, auth by line only

Every bad file is logged as `<file>: <message>`. Config and records use the
`toml` or `toml_edit` error text, which already names the line. So does the
record store's own wrong-type message. The config log line drops its old
`config reload failed: the configuration file is not valid:` prefix.

The `toml` error text quotes the offending line. For `auth.toml` that would put
a key into the log, so auth builds its message from the error's span only, as
`auth.toml: line <n> isn't valid TOML`. This is a trap worth a line in
AGENTS.md.

### `version` in keyed files

`version` stays reserved at the top level of `snippets.toml`,
`quicklinks.toml`, and `auth.toml`, because AGENTS.md keeps version fields. It
is never projected as a record or a provider. The schema allows it next to the
open id keys.

### Strings the user sees

Only file names change in on-screen text. The log lines change as above.
Checked against `interface-copy`:

| Where | Before | After |
|---|---|---|
| Record parse error (log, tray) | `snippets.json: The file isn't valid JSON: <message>` | `snippets.toml: <message>` |
| Record wrong type (log, tray) | none, JSON ignored it | `snippets.toml: Line <n>: name must be text` |
| Record file broken | `Fix the error in snippets.json first` | `Fix the error in snippets.toml first` |
| Record write failed | `Couldn't save, check that snippets.json isn't read-only` | `Couldn't save, check that snippets.toml isn't read-only` |
| Startup notice | `snippets.json has an error, see dango.log` | `snippets.toml has an error, see dango.log` |
| Config parse error (log) | `config reload failed: the configuration file is not valid: <message>` | `config.toml: <message>` |
| Auth parse error (log) | line only | `auth.toml: line <n> isn't valid TOML` |
| Auth unreadable | `auth.json couldn't be read. Check the file in your config folder.` | `auth.toml couldn't be read. Check the file in your config folder.` |

No contract change: the extension manifest and view protocol are untouched.

## Risks / Trade-offs

- [Usage ranking for existing records resets, because ids change] → Accepted
  for an alpha. The hand conversion keeps the readable ids that already exist
  (`github`, `signature`) and slugs the UUID ones.
- [A wrong field type now makes a record file invalid where JSON ignored it] →
  Same rule as `config.toml`. The error names the line, and the last good
  records stay.
- [New entries' formatting may differ from the user's style, for example
  inline vs. standard tables] → New entries are always standard `[id]` tables;
  existing ones are left as the user wrote them.
- [The old `config.schema.json` and the JSON files stay in the author's config
  folder] → Removed by hand during the conversion task.

## Migration Plan

No code migration. Steps for the author's machine, done once by hand:

1. Convert `snippets.json`, `quicklinks.json`, and `auth.json` to TOML keyed
   tables.
2. Change the first line of `config.toml` to
   `#:schema ./.schemas/config.schema.json`.
3. Delete `config.schema.json` and the old JSON files.

Rollback: an older build still reads the JSON files if they are kept until the
new build is in daily use.
