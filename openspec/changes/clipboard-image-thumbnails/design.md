## Context

See proposal.md - Why. Today:

- `History::record` runs on the watcher's own thread and writes each image as
  `<id>.png` in the images directory. The row stores its path; the `bytes`
  column feeds the size bound.
- The image row's icon is that path (`extension.rs`, `item`).
- `Icons::get` caches named icons only. A file icon is decoded with
  `Image::load_from_path` on every render, and every snapshot replaces the
  list model.
- `History::new` deletes any `.png` in the images directory that no row
  points to.

## Goals / Non-Goals

**Goals:**

- A key press that does not change which rows are shown decodes no image.
- The images a render does decode are small.
- Idle memory does not grow with the history.

**Non-Goals:**

- Off-main-thread decoding in the renderer. Small thumbnails plus the cache
  make it unnecessary.

## Decisions

### 1. A thumbnail file made at record time

`record` decodes the PNG it already holds, makes a thumbnail that fits in
384 x 384 with `RustImageData::thumbnail` (aspect ratio kept, never enlarged),
and writes it with `to_png` as `<id>.thumb.png` beside the full image.

- **Where:** in `record`, which runs on the watcher thread, so decoding a 4K
  screenshot never touches the main thread. The existing "a large image does
  not stall the launcher" requirement still holds.
- **Library:** `clipboard-rs` already wraps `image`, and its `thumbnail` uses
  `image`'s fast integer sampler. No new crate. A finer filter
  (`resize` with Triangle or Lanczos) is not needed for a downscale this size
  and costs more.
- **Size, 384 px:** a grid tile in a 5-column launcher is about 140 logical
  px, which is about 280 physical px at 200 %. 384 px stays sharp there and in
  any list row, so a later grid or a bigger row needs no new size. 64 px
  would look blocky in a grid.
- **Name, not column:** the thumbnail path is derived from the image path, so
  the table does not change. `Entry` gains a `thumbnail` that is the derived
  path when that file exists.
- **Failure:** if the bytes do not decode or the write fails, the entry is
  still recorded with no thumbnail and the failure is logged. The row then
  falls back to the full image, as entries recorded before this change do.

### 2. Thumbnails share the image's lifecycle

- The `bytes` column counts the image plus its thumbnail, so the size bound
  stays honest.
- `remove`, `trim`, and the "file has gone" path delete both files.
- The orphan sweep keeps `<id>.thumb.png` when `<id>.png` belongs to a row and
  deletes it otherwise.

### 3. A render-scoped file-image cache

`Icons` keeps the decoded `Image` of each file path the last render used.
A render marks each path it asks for; at the end of the render, paths not
marked are dropped. The cache is emptied, together with the list and grid
models, when the launcher hides, since those models also hold the `Image`s.

- Arrow Down, typing that keeps rows, and the action panel decode nothing.
- Memory is bounded by what one screen shows: at most 50 thumbnails of at
  most 384 x 384 RGBA, about 30 MB in the worst case, and only while open.
- It keeps A2-01's point: no image outlives the screen that shows it.

Alternatives:

- **Revert A2-01 (cache forever):** fast, but memory grows with every
  screenshot, a 4K one being about 33 MB of pixels.
- **Bounded LRU of downscaled images, no stored thumbnail:** no stored-data
  change, but every open after a restart or eviction decodes full-size images
  on the main thread, the very cost being removed. It also needs a size limit
  tuned by hand.
- **Rely on Slint's image cache:** about 5 MB and not configurable from the
  public API, so full-size screenshots fall out at once.

### 4. Optimise image decoding in debug builds

`Cargo.toml` gets `[profile.dev.package.<name>] opt-level = 3` for `png`,
`fdeflate`, `miniz_oxide`, and `image`. Unoptimised inflate is a large part of
the lag the hand check measured in a debug build, and hand checks run in debug
builds. Release builds are unaffected.

No view protocol or extension manifest change: the row still carries an icon
path.

## Risks / Trade-offs

- [The first open after a restart still decodes up to 50 thumbnails on the
  main thread] -> They are at most 384 px, so this is a short one-off, not a
  per-key cost. Measure it in the Windows hand check.
- [Entries recorded before this change keep the full-size cost] -> Only until
  they roll out of the history; alpha allows no backfill. Clearing the history
  removes them at once.
- [A file icon path that changes content under the same name shows a stale
  image while the launcher stays open] -> Paths are per entry id and never
  rewritten. Hiding empties the cache.
- [384 px PNG thumbnails add disk use] -> Roughly 50-150 KB each, counted in
  the size bound.
