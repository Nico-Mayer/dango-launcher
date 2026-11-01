## Why

Clipboard History shows each entry as one cut-off line or a small thumbnail.
You cannot read a longer text, see an image at a useful size, or tell when or
where something was copied without pasting it first.

## What Changes

- Clipboard History becomes a list with a detail pane. The pane shows the
  selected entry in full: the text as it was copied, or the image at pane size.
- Below the body, the pane shows metadata: where it was copied from, when, what
  kind of content it is, and its size (lines and characters, or pixels).
- The source application is recorded with each entry. Copying the same thing
  again updates its source and time.
- Text that looks like a link, an email address, a colour, or a file path is
  labelled as such in the metadata.
- Each image gets a second, larger preview made when it is recorded, so the
  pane never decodes the full image.
- **BREAKING** (view protocol): a detail, standalone or in a list's pane, can
  show an image above its body and can carry plain text instead of markdown.
  No protocol version bump: every producer is a built-in.
- **BREAKING** (stored data): the clipboard history table gains columns for the
  source application and image size. No migration of old rows.

## Non-goals

- New kinds of content, such as files or rich text. That is the next change;
  its macOS part is tracked in #9.
- Showing the source application's icon.
- Actions on detected kinds, such as opening a link or showing a colour swatch.
- Editing an entry, pinning, or search by metadata.

## Platforms

Windows and macOS. The source application differs by platform: Windows names
the copying application exactly, so it is always shown when known. macOS can
only name the applications that were frontmost around the copy, so the source
is shown only when there is exactly one. Built and checked on Windows in this
session; the macOS hand checks stay open in `tasks.md`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `clipboard-history`: entries are shown in a detail pane with metadata; the
  source application is recorded; image previews stay responsive in the pane.
- `view-protocol`: a detail can show an image and plain text.
- `design-system`: the detail layout places an image above the body and shows
  plain text in the monospace face.

## Impact

- `src/protocol.rs`: `ItemDetail` and `DetailView` gain `image` and `text`.
- `src/ui/snapshot.rs`, `src/slint_shell/render/detail.rs`,
  `ui/views/detail.slint`, `ui/globals.slint`: render the image and plain text.
- `src/extensions/clipboard/`: the history view, a new migration, recording the
  source and image size, the preview file, detection of kinds.
- `src/platform/macos/clipboard.rs`, `src/platform/windows/clipboard.rs`:
  attribution is asked on every recorded change, not only when the exclusion
  list is set.
- No new dependency: `time` and `url` are already in the tree.
