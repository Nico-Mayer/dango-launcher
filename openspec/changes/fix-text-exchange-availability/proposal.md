## Why

On Windows, AI commands fail with "Dango needs the Accessibility permission to
paste. Grant it in System Settings, Privacy & Security." Windows has no such
permission. The message is the fallback every extension shows when the text
exchange, the one path for pasting and reading the selection, was never built.
That happens when the database is unavailable, when the clipboard cannot be
opened, or when key injection cannot start. The message hides the real cause,
and the database case breaks AI paste for a reason unrelated to it.

macOS has the same gap, worse. Key injection refuses to start without the
Accessibility permission, so a Dango started without it has no text exchange at
all. Granting the permission afterwards changes nothing until Dango restarts,
which the selection-and-paste spec already rules out.

## What Changes

- A failure to build the text exchange gets its own message, separate from the
  missing permission. The permission message is shown only when the permission
  check says it is missing, so Windows never shows it.
- The text exchange no longer depends on the clipboard history database. Without
  the database, history is off, but pasting, snippets, and AI commands still
  work.
- On macOS, key injection is set up on first use instead of at startup, so the
  exchange always exists and a permission granted after startup works without a
  restart.
- When the text exchange cannot be built, the tray shows a notice naming
  `dango.log`, as other startup problems do.

Milestone: a fix within M3 plumbing, where the text exchange was built. It also
unblocks the permission warning planned for M7.
Platforms: Windows and macOS. The message and the database decoupling are in
platform-neutral code and apply to both. Lazy key injection is macOS only,
because Windows key injection needs no permission and does not fail on it.

## Non-goals

- The permission warning itself: a root item, a "Grant access" action, a tray
  line for the missing permission, and live re-checks. That is the next change.
- Restarting keyword expansion or the hyperkey after a grant. Both depend on an
  event tap, not on the text exchange, and belong to the permission change.
- Input Monitoring on macOS.
- Elevated target windows on Windows. The existing message stays.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `selection-and-paste`: "The macOS permission is a visible state" gains a
  scenario for a permission granted after Dango started. "Failures are reported,
  never silent" gains scenarios for a text path that could not be set up, with
  a truthful message and a tray notice, and for a missing database leaving
  pasting intact.

## Impact

- `src-tauri/src/text/mod.rs`: a new `TextError` variant for an unavailable text
  path.
- `src-tauri/src/extensions/ai/extension.rs`, `clipboard/extension.rs`,
  `snippets/extension.rs`: use the new variant when no text target exists, and
  their tests.
- `src-tauri/src/lib.rs`: open the clipboard and build the exchange outside the
  database branch, with `Unwatched` when there is no history; push a tray
  notice when it fails.
- `src-tauri/src/platform/macos/text.rs` and `platform/mod.rs`: `MacKeys`
  creates enigo lazily on the main thread.
- No change to the extension manifest, the view protocol, the config file, or
  stored data.
