## Why

Copying something and then copying something else loses the first thing. A
clipboard history is the feature people reach for most in a launcher of this
kind, and it is the last item in **M2 - builtins** from `openspec/ROADMAP.md`.

It is also the first extension that needs three things the contract declares but
has never exercised: a background service that runs for the whole session, a
preference an extension reads at run time, and binary content that is not a
string. `applications` and `system` prove commands, views, and services that
only index; none of them prove a service that observes the world and writes to
the database while the user works.

## What Changes

- A `clipboard-history` built-in extension with a watcher service that records
  what the user copies, text and images alike, for as long as Dango is running.
- A command that opens the history, newest first, narrowable by typing, with the
  chosen entry put back on the clipboard.
- Bounded retention: a fixed number of entries, with a size ceiling so a run of
  screenshots cannot fill the disk. The oldest go first.
- Privacy exclusions, so a password manager's clipboard never reaches the
  history. Driven by the markers the platforms already provide and by a list of
  applications the user can add to.
- Preference values become readable and writable in the store. The typed schema
  and the database table have existed since M1 and nothing has ever read one;
  the exclusion list is the first feature that needs it.
- Covered on Windows and macOS.

## Capabilities

### New Capabilities

- `clipboard-history`: what is recorded and what is deliberately not, how long
  it is kept and what is dropped when the bounds are reached, how an entry is
  found and put back on the clipboard, and how images differ from text.

### Modified Capabilities

- `local-store`: adds the requirement that preference values are readable and
  writable, and that reading an unset preference yields the extension's declared
  default. `extension-model` already requires that behaviour of the preference
  schema; this puts the persistence half under the capability that owns the
  database.

## Impact

- **Affected code**: `src-tauri/src/extensions/clipboard/` (new),
  `src-tauri/src/platform/` (reading the clipboard and its privacy markers is
  per platform, behind a trait), `src-tauri/src/store/` (preference read and
  write, plus the history tables), a new migration.
- **Database**: a forward-only migration adding the history. The history is
  machine-local and never syncs, so it follows the `local_` convention rather
  than the syncable one. Copying is not something two machines should agree on,
  and a synced clipboard history is a privacy hazard rather than a feature.
- **Dependencies**: an image encoder is already present (`png`). Reading the
  clipboard is expected to need no new crate on either platform.
- **Frontend**: no new rendering. The history is a list view with icons, which
  the existing renderer already handles.

## Non-goals

- **No paste.** Choosing an entry puts it back on the clipboard; the user pastes
  it themselves. Pasting into the frontmost application needs the selection and
  focus-restore plumbing M3 builds, and pulling it forward would mean building
  that plumbing here with one consumer to guess against.
- **No preferences user interface.** Values become readable and writable, and
  the exclusion list is edited in the database until M7 renders the schema. The
  alternative is designing M7's preferences window now, against one extension.
- **No pinning, tagging, or search across history content.** Narrowing by what
  is visible in the list is enough to find something copied minutes ago, which
  is what the feature is for.
- **No history sync.** See the database note above, and M9.
- **No rich text or file lists.** Plain text and images only. A third content
  type can be added when something needs it; two is enough to prove that the
  store is not text-only.
- **No history in root search.** It opens through its own command. Every entry
  competing with applications in root search would drown the launcher's main
  surface in things the user copied.
- **No Linux.** Out of scope for the project.
