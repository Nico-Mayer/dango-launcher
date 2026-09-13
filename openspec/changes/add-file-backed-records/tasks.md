## 1. The file-backed store

- [x] 1.1 Define the on-disk record shape `{ id, name, body }` with `id` optional on read, tolerating and preserving unknown fields, and parse and serialise an array of them, with tests over a round trip and over an entry missing its id
- [x] 1.2 Rework `Records` to hold the file path (derived from the kind and the config directory), the kind, and an in-memory cache loaded from the file, keeping `create`, `update`, `remove`, `all`, and `get` behaving as before but against the cache, and dropping the `now` timestamp parameters
- [x] 1.3 Assign an id to any record that has none when the store writes the file, with a test that a hand-authored entry without an id gains one and keeps its meaning
- [x] 1.4 Persist every mutation to the file and record the app's own write for the watcher, with tests that create, update, and remove change the file and that remove deletes the entry rather than tombstoning it
- [x] 1.5 Keep the existing validation and error behaviour (no name, no body, unparseable template, and acting on a missing record), with the tests carried over from the SQLite store
- [x] 1.6 Keep the last good cache and surface the error when the file does not parse, with a test that a bad file does not clear the records

## 2. Wiring and migration

- [x] 2.1 Build the snippets and quicklinks stores against the config directory in `lib.rs` instead of the database, and verify the extensions load and their commands work
- [x] 2.2 Watch each record file with the config watcher, reloading the cache on an external edit and ignoring the app's own writes, applied live
- [x] 2.3 Add a forward-only migration dropping the `snippets` and `quicklinks` tables, and confirm it applies to an existing database without touching the other tables
- [x] 2.4 Verify the existing test suite passes and nothing else read those tables

## 3. Verification

- [ ] 3.1 Confirm on both platforms that creating a snippet and a quicklink through the launcher writes them to `snippets.json` and `quicklinks.json` in the config directory as readable text
- [ ] 3.2 Confirm on both platforms that a record hand-added to the file appears in the launcher, gains an id on the next app write, and that editing a record's body live changes what the launcher uses without a restart
- [ ] 3.3 Confirm on both platforms that removing a record deletes its entry and it stays gone after a restart, and that a malformed file keeps the last good records and surfaces the error
- [ ] 3.4 Confirm on both platforms that the records travel: copying the config directory to a second location via `$DANGO_CONFIG_DIR` brings the snippets and quicklinks with it
