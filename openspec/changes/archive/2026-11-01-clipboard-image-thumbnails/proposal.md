## Why

Clipboard History with many image entries lags 1-2 s on every key press (#8).
Each snapshot replaces the list, and each image row decodes its full-size PNG
on the main thread. `246a9f7` (audit A2-01) stopped caching file icons to stop
memory growth, which moved the cost from memory to decode time.

## What Changes

- When an image is recorded, the history also stores a small thumbnail of it
  beside the full image. Rows show the thumbnail. The full image is still what
  is pasted and copied.
- The renderer keeps the file images the current snapshot shows, so a key
  press that does not change the rows decodes nothing. It lets go of them when
  the launcher hides.
- Debug builds optimise the image decoding crates, so hand checks in a debug
  build show realistic speed.
- Stored data changes: image entries gain a thumbnail file. Entries recorded
  before this change get none and fall back to the full image. No migration
  (alpha, see `AGENTS.md`).

## Non-goals

- No detail pane or grid layout for Clipboard History. The thumbnail size
  leaves room for a grid later, but this change does not build one.
- No change to the view protocol or the extension manifest.
- No backfill of thumbnails for existing entries.
- No general file-icon cache across hides, and no change to how application
  icons are extracted.
- No fix for the open question in #8 about how many list rows the element
  tree holds.

## Platforms

Windows and macOS. The thumbnail is made in platform-neutral code from the PNG
bytes the watcher already has, and the cache lives in the shared renderer.
Windows is verified in this session; the macOS hand check stays open until a
session on the Mac.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `clipboard-history`: browsing image entries stays responsive, and leaving
  the history gives back the memory its images used.

## Impact

- `src/extensions/clipboard/history.rs`: writes, counts, and deletes the
  thumbnail; the orphan sweep keeps thumbnails of live entries.
- `src/extensions/clipboard/extension.rs`: the row icon is the thumbnail.
- `src/slint_shell/icons.rs`, `src/slint_shell/render/`, and the shell's hide
  path: the render-scoped file-image cache.
- `Cargo.toml`: a `[profile.dev.package]` entry for the image decoding crates.
- No new dependency: `clipboard-rs` already exposes `thumbnail` and `to_png`.

Refs #8
