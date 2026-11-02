# Design

## Context

The history knows two kinds, text and image (`Content`, `Kind` in
`src/extensions/clipboard/history.rs`). The watcher reads text first, then an
image (`Watcher::read`). Files already appear in one place: the `Snapshot` a
paste saves and restores, through `clipboard-rs`'s `ContentFormat::Files`
(`source.rs`).

What the platforms put on the clipboard for a file copy:

- macOS Finder: file URLs plus the file names as plain text. Today this is
  recorded as a text entry of the names.
- Windows File Explorer: `CF_HDROP` (plus shell formats), no plain text. Today
  nothing is recorded.

`clipboard-rs` 0.3.5 already reads and writes files on both:

- macOS `get_files` reads `NSURL`s and returns local paths; `set_files` writes
  `NSURL`s through `writeObjects` and drops paths that do not exist, failing
  only when none are left.
- Windows `get_files` reads `CF_HDROP` (`FileList`); `set_files` writes it with
  `clipboard-win`. It does not check that paths exist.

`platform::icon_for` already returns the system icon for a filesystem path on
both platforms (`NSWorkspace iconForFile` on macOS; on Windows a path is a valid
shell parsing name for `IShellItemImageFactory`). `IconCache`
(`extensions/applications/icons.rs`) turns it into a cached file Slint can load.

## Goals / Non-Goals

**Goals:**

- Files as a third `Content` and `Kind`, through record, list, pane, restore.
- Own-write suppression and the privacy checks cover files with no new rule.

**Non-Goals:**

- Watching the files for moves or renames. A path is checked only when the
  entry is used.

## Decisions

### `Content::Files(Vec<String>)`, stored as newline-joined paths in `text`

The row keeps the paths in the existing `text` column, one per line, with kind
`files`. Fingerprint is over `files\0` plus the joined paths, so the same
files in the same order reorder rather than duplicate. `bytes` is the length
of the joined paths: an entry costs what it stores, not the size of the files.

- Rejected: a new `paths` column or a side table. One more schema change for
  no gain; a path never contains a newline on Windows, and on macOS a file
  name with a newline is rare enough to accept splitting wrongly.
- Rejected: copying the files into the cache. Costs disk and time in the
  watcher, and the bounds would have to count file sizes. Listed as non-goal.

`entries()` cuts `text` to `LISTED_CHARS` for listing. A file entry needs every
path for keywords and pane, so the list query does not cut rows of kind
`files`. Paths are short; 500 entries of paths is small.

### Files are read before text

`Watcher::read` asks for files first, then text, then an image. A Finder copy
offers both files and names as text, and the files are what the user meant.
This is the one place where the recorded result of an existing copy changes.

- Rejected: text first, files only when no text. Keeps Finder copies as names,
  which is the bug.

### `ClipboardSource` gains `files()` and `set_files()`

`CrateClipboard` maps them to `get_files` and `set_files`, through `until_free`
like text and images. `files()` returns `None` for an empty list. The fake
sources in tests get the same two methods.

`Snapshot::content` returns `Content::Files` when the snapshot has files, ahead
of text, so restoring a clipboard that held files after an insertion matches
what the watcher now reads. Without this, the restore would be recorded as a
new entry.

### Missing files are filtered by Dango, not by the crate

`History::content` for a file entry keeps only paths that exist
(`Path::exists`). None left: the entry is removed and `HistoryError::FilesGone
{ count }` is returned, so the message can say "file" or "files". The crate's
macOS filter is then a no-op, and Windows behaves the same, which it would not
if the check were left to the crate.

Checking before the paste also keeps "the clipboard is unchanged" true: no
write happens when nothing is left.

### Writes go through the existing paths

`Watcher::restore` and `TextExchange::write` gain a `Content::Files` arm that
calls `set_files`. `paste_content` needs no other change: it writes, then sends
Cmd+V or Ctrl+V, and Finder and File Explorer paste files from that.

Own-write matching stays exact for files: the paths that go out are the paths
that come back. Order is kept by both crates. If a platform turns out to
normalise paths (for example macOS resolving `/private/var`), the match fails
and the restore is recorded once; the hand check covers this.

### List item, pane, icons

- Title: the first file's name (`Path::file_name`, falling back to the whole
  path), plus " and N more" for several. Keywords: every file name, so
  narrowing finds the entry by any of them.
- Icon: `platform::icon_for` on the first path, cached in an `IconCache` held
  by the clipboard extension, the way the system extension does for running
  apps. The cache is keyed by path. No icon (path gone, render failed) falls
  back to `icon:file`, a new name in the registry (`lu::LuFile`).
- Pane: `ItemDetail.text` with the paths, one per line. The text pane is
  `ui/views/` detail view, already used for text entries; no new component.
- Metadata: "Type" "File" or "Files"; "Count" for more than one. A folder is
  "File" too: telling folders apart means a filesystem call per render for a
  word.

Rendering the icon calls the platform on the main thread when the history
opens, once per new path. This is what the system extension already does for
running apps. If it shows as lag with many file entries, icons move to the
background like app icons, but that is not built now.

### Copy

Every new or changed string, written with the `ux-writing` skill and checked
against `openspec/specs/interface-copy/spec.md`:

- Title, several files: "report.pdf and 2 more"
- Type values: "File", "Files"
- Metadata label: "Count"
- Empty state description: "Text, images, and files you copy will show up
  here" (was "Text and images you copy will show up here")
- Failure, one file: "The copied file no longer exists"
- Failure, several: "The copied files no longer exist"

The failures follow the interface-copy rule that a thing that no longer exists
says so, in one sentence, with no path or OS error.

### Contracts

No change to the extension manifest or view protocol: file entries use
`ListItem.title`, `keywords`, `icon` and `ItemDetail.text` and `metadata` as
they are.

## Risks / Trade-offs

- [A Finder copy that also carries meaningful text, from an app that puts
  files and text on the clipboard] → Recorded as files. Accepted: the files
  are the richer content, and nothing common does this.
- [Path normalisation breaks own-write matching] → Caught by the macOS hand
  check; if it shows, match on canonicalised paths.
- [Icon rendering on the main thread with many file entries] → Same pattern as
  running apps; move to background only if a hand check shows lag.
- [File Explorer may paste a bare `CF_HDROP` differently from its own copy,
  not verified] → Covered by the Windows hand check.

## Migration Plan

None. Alpha: existing rows keep working, new rows have kind `files`.
