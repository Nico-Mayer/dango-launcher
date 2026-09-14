## Context

See proposal.md for why. What already exists and shapes this:

- `Records` (`extensions/snippets/store.rs`) is a concrete struct over `Store`,
  keyed by `Kind` (Snippet or Quicklink), with `create`, `update`, `remove`,
  `all`, and `get`. A record is `{ id, name, body }`; the table also carries
  `created_at`, `updated_at`, and `deleted_at` from the syncable convention. The
  snippets and quicklinks extensions each hold one `Arc<Records>`.
- `add-config-file` established the config directory, a lossless JSON load, a
  file watcher (`config::watch::watch`) with debounce and own-write suppression,
  and the pattern of keeping the last good data and surfacing errors to a tray
  line and a log. This change reuses all of that.
- The root provider reads `records.all()` per query, against a 50ms budget with
  up to 500 records.

## Goals / Non-Goals

**Goals:**

- Authored records become plain, git-friendly, hand-editable text in the config
  directory, live-editable, with the extensions above them unchanged.
- Reuse the config change's watcher, own-write, and last-good machinery rather
  than inventing a second copy.

**Non-Goals:**

- See proposal.md. At design level additionally: no merge or conflict resolution
  (git's job), and no in-file schema versioning beyond tolerating extra fields.

## Decisions

### One file per kind, an array of records

`snippets.json` and `quicklinks.json`, each a JSON array of `{ id, name, body }`.

Rejected: one file per record in a directory (`snippets/<id>.json`). It merges
better, since editing one record cannot conflict with another, which is a real
advantage for git. But it is a directory of opaque files rather than one list a
person can open, read, and reorder, and this is a single author on two machines
where whole-file conflicts on a short list are rare and trivial to resolve. The
single file wins on being the thing you actually want to read and edit, and it
matches `config.json` being one file. If cross-machine conflicts ever bite, the
per-record layout is the escape hatch, and the store is written so that switch is
localised.

This is the main choice worth a second look; it is easy to change later because
nothing above the store sees the file layout.

### A record is `{ id, name, body }`, no timestamps, no soft delete

The syncable columns existed for a sync engine that reads timestamps and tombstones.
Git is that engine now: history and deletion are the repository's, not the
record's. So `created_at`, `updated_at`, and `deleted_at` are dropped. Removing a
record deletes its array entry; it does not come back because there is no tombstone
to resurrect and git remembers what was removed.

Dropping `updated_at` also keeps diffs clean: editing one snippet changes one
entry, not a timestamp on every save.

`id` stays, for stable identity when the launcher edits or removes a specific
record. It is optional in the file: a hand-authored entry may give only a name and
a body, and the store assigns an id the next time it writes the file. Removing the
`now` timestamp arguments simplifies the store's API and its call sites.

### The store keeps its shape but reads and writes files

`Records` is reworked from SQLite to a file backing without changing what the
extensions call. It holds the file path, the kind, and an in-memory cache behind a
lock, loaded from the file at startup. `all` and `get` read the cache, so the per
keystroke root provider stays fast and never parses the file on the hot path.
`create`, `update`, and `remove` mutate the cache and write the file. This mirrors
`FileConfig`: the cache is the live view, the file is the durable form.

Rejected: reading and parsing the file on every `all()`. Simple, but it puts file
IO and JSON parsing on the search path the project keeps under 50ms, and it would
reread the whole file on every keystroke.

### Edits apply live through the config watcher, reused

Each record file is watched with `config::watch::watch`, the same debounced
watcher with own-write suppression the config file uses. On an external edit the
cache is reloaded from the file; on the app's own write the change is ignored.
Because the extensions read the cache, a reloaded cache is immediately live.

### A bad file keeps the last good records and is surfaced

An unparseable record file does not clear the cache: the last good set stays, the
error goes to the log and the tray status line, and the app keeps running. This is
the config change's rule applied to records, so a fat-fingered edit never destroys
the user's snippets.

### The SQLite tables are dropped, with no export

A migration drops `snippets` and `quicklinks`. The feature is weeks old and the
author has few if any records in SQLite, so a one-time exporter is not worth
building; the tables go and the files start empty. This is stated rather than
assumed, so if there is real data to keep, the call can be revisited before the
migration lands.

## Risks / Trade-offs

- **Whole-file merge conflicts across machines.** Two machines editing different
  snippets can still conflict on the one array. → Accepted for a single author with
  a short list; the per-record layout is the documented escape hatch.
- **App write racing a hand edit.** A save while the file is being edited. → The
  own-write suppression and last-good handling from the config change absorb it,
  and the cache means a transient bad read does not disrupt the running app.
- **Coordination with `add-keyword-expansion`.** It adds a `keyword` to a snippet,
  also in the store. → The record is an object with room for more fields, and the
  loader tolerates extra keys, so whichever lands second adds the field without a
  format break.
- **Losing the syncable timestamps.** M9 might have wanted them. → M9 is
  file-based sync, and files in git already carry history and deletion, so the
  timestamps were machinery for a design this supersedes for authored content.
