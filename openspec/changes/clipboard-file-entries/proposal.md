# Proposal

## Why

Files copied in Finder or File Explorer are lost from the clipboard history.
On Windows nothing is recorded. On macOS only the file name is kept as text,
so pasting it back gives a name, not the file. Tracks issue #9.

## What Changes

- Files copied in a file manager are recorded as one history entry that holds
  their paths. The files themselves are not copied or kept.
- A file entry is listed by its file name, with the icon the system shows for
  the file, and the pane lists every path.
- Pasting or copying a file entry puts the files back on the clipboard, so a
  file manager or another app takes them as files.
- If some files no longer exist, only the ones still there go back. If none
  exist, the failure is reported and the entry is removed.
- Concealed and excluded content, and Dango's own writes, are skipped for files
  as for text and images.
- On macOS, a Finder copy is now recorded as files rather than as its file name
  as text.

## Non-goals

- Rich text (HTML, RTF) entries. Issue #9 mentioned them as possible; they
  stay out.
- Keeping copies of file contents, so an entry still works after the file is
  deleted or moved.
- Previews of file contents (Quick Look, thumbnails of images or PDFs) in the
  list or the pane. The system icon is enough.
- Folder size or file size metadata.
- Changes to how a paste saves and restores the clipboard. Files already
  survive that round trip.

## Platforms

Windows and macOS. Built and checked on macOS in this session. The Windows
hand checks stay open tasks until a session on that machine, and the change is
not archived before them.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `clipboard-history`: files become a third recorded kind, with their own
  listing, pane, metadata, restore and missing-file behaviour.

## Impact

- `src/extensions/clipboard/`: `Content`, `Kind`, history storage, watcher
  read order and own-write matching, list item, detail pane, icons.
- `ClipboardSource` gains file read and write; `CrateClipboard` uses
  `clipboard-rs`'s existing file support.
- `src/text/`: `paste_content` writes a file entry; `Snapshot::content`
  reports files so a restore is recognised as Dango's own.
- Database: rows of kind `files`. No migration (alpha).
- No change to the extension manifest or view protocol.
