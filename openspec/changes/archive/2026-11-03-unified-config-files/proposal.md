## Why

`config.toml` now has a schema, a style, and clear error handling, but the
other files in the config directory do not. Snippets, quicklinks, and API keys
are still JSON arrays and maps with UUID ids, no schema, and no shared rules.
Even for `config.toml`, the rules for live reload, bad files, and writes are
stated separately from the same rules for records, and the code handles each
file its own way. One contract for every file the user reads or edits makes
them feel like one design, gives each one editor support, and gives future
files a rule to follow.

## What Changes

- New contract for every user-facing file in the config directory: TOML
  format, one key style, user-named entries as tables keyed by id, a generated
  schema per file, a clear owner (user only, or user and Dango), live edits,
  and one way to report a bad file.
- **BREAKING** `snippets.json` and `quicklinks.json` become `snippets.toml` and
  `quicklinks.toml`. Each record is a table keyed by its id, a readable slug
  made from its name, instead of an array entry with a UUID. The JSON files
  are no longer read. Usage ranking for existing records starts over, because
  the ids change.
- When Dango writes a record file, it keeps the user's comments, order, and
  unknown keys. A field with the wrong type makes a record file invalid, as it
  already does for `config.toml`.
- **BREAKING** `auth.json` becomes `auth.toml`, still a flat map of provider
  name to key. A parse error names the line, never the line's text.
- **BREAKING** Schemas move into a `.schemas/` folder in the config directory,
  one per file (`config.schema.json`, `snippets.schema.json`,
  `quicklinks.schema.json`, `auth.schema.json`), all made by one generator.
  The `#:schema` line in a created `config.toml` points into the folder.
- `config.toml` joins the same code paths: one list of user files for paths,
  headers, and schemas, one write helper, and one log format for a bad file.
- Rules that `configuration` and `records-storage` each stated (style,
  versioning, live edits, bad files, who writes) move into the contract.
- The settings reference `docs/config.md` covers every file.
- Remaining references to JSON config files are removed: the stale
  `config.json` read in `examples/macos_keys_walkthrough.rs`,
  `docs/auth.example.json`, and file names in README, AGENTS.md, doc comments,
  and tests.

## Non-goals

- No new feature: no new record fields, no aliases or hotkeys for records, no
  new settings.
- No change to how snippets, quicklinks, or providers behave beyond the files
  they are stored in.
- No migration or conversion code. The author converts their own files once by
  hand.
- No change to `config.toml`'s keys or structure.
- No move of secrets to the OS keyring.

## Platforms

Platform-neutral: the same files, paths, and behaviour on Windows and macOS.
The only platform difference already in place, restricting `auth` to its owner
on macOS, carries over unchanged.

## Capabilities

### New Capabilities

- `config-files`: The contract every user-facing file in the config directory
  follows: format, location, schema folder and header line, key style, keyed
  collections, versioning, ownership and writing, live edits, and bad-file
  handling.

### Modified Capabilities

- `configuration`: Points to `config-files`; its versioning, live edit,
  bad-file, and write rules move there.
- `config-schema`: One generator writes a schema per file into `.schemas/`;
  the editor rule moves to `config-files`; the settings reference covers every
  file.
- `records-storage`: TOML files keyed by id, slug ids, no more "assign an id
  later"; live edit and bad-file rules move to `config-files`.
- `ai-providers`: Keys read from `auth.toml`; a parse error shows the line,
  never file text.
- `system-commands`: Open Config File creates a file whose header points into
  `.schemas/`.

## Impact

- Code: `src/config/` (user file list, write helper, generalised schema
  generator, log format), `src/extensions/snippets/store.rs` (TOML via
  `toml_edit`), `src/extensions/ai/auth.rs`, `src/app/startup.rs`,
  `src/app/mod.rs`, `src/browser.rs` and other doc comments, tests.
- Dependencies: `toml_edit` becomes a runtime dependency (today only a build
  dependency through Slint), and the `slug` crate is added.
- Docs: `docs/config.md`, `docs/auth.example.toml`, README, AGENTS.md.
- User data: the author's `snippets.json`, `quicklinks.json`, and `auth.json`
  converted by hand, and the `config.toml` header changed.
