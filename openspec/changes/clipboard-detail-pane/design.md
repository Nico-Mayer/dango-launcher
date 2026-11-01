## Context

See proposal.md - Why. Today:

- The view protocol already has a list with a detail pane (`ListView::detail_pane`,
  `ListItem::detail`), drawn by `ui/views/list.slint` with the `Detail` view
  from `ui/views/detail.slint`. No extension uses it yet.
- A detail body is markdown only, rendered with `StyledText::from_markdown`.
  It cannot show an image.
- `History::record` already decodes each image on the watcher thread to write a
  384 px `<id>.thumb.png` (archived change `clipboard-image-thumbnails`).
- `Icons` keeps a render-scoped cache of decoded file images; the detail writer
  does not use it yet.
- The watcher asks `Attribution::candidate_applications` only when the
  exclusion list is not empty. Both platforms answer with display names.
- With a pane the launcher is at full height (480 logical px), and the pane is
  about 400 logical px wide.

## Goals / Non-Goals

**Goals:**

- Moving the selection decodes at most one small image, never a full one.
- The metadata needs no extra disk read per entry when the history opens.

**Non-Goals:**

- Off-main-thread image decoding in the renderer. A bounded preview plus the
  cache keeps the cost small, as the thumbnail change found.
- A protocol version bump. Every producer is a built-in (see `PROTOCOL_VERSION`).

## Decisions

### 1. Protocol: `image` and `text` on a detail

`ItemDetail` and `DetailView` both gain `image: Option<String>` (a named icon
or a path, as everywhere else) and `text: Option<String>` (plain text). When
`text` is set it is the body and `markdown` is ignored. `DetailBody` in the
snapshot carries both.

This changes the view protocol, a versioned contract: a detail now has three
optional parts, and plain text wins over markdown. Both fields default to
`None`, so the AI commands' detail views are unchanged.

Alternatives:

- **Escape the text into a markdown code block.** Slint's markdown support is
  limited and its code block styling is not ours to control. Escaping every
  markdown character is fragile and still renders some constructs.
- **Replace `markdown` with an enum `Markdown | Text`.** Cleaner, but churns
  every producer for no behaviour gain. Breaking changes are free, but this one
  buys nothing.
- **A separate image item kind or a grid for images.** Splits the history into
  two shapes for one list.

### 2. The `Detail` view draws the image and plain text

`ui/views/detail.slint` stays the one detail part of the design system. It
gains, above the body:

- An `Image` with `image-fit: contain`, framed like other image previews
  (`Theme` radius and border). Its width is the smaller of the pane's width and
  the image's own logical width, so it is never enlarged; its height is capped
  at a new `Theme.detail-image-height` (200 px) so the metadata stays in view
  on a 480 px launcher. Accessible role `image`, label "Image".
- For a plain text body, a `Text` in `Theme.mono` with `wrap: word-wrap`,
  instead of the `StyledText`.

`DetailData` gains `image`, `has-image`, and `plain`. The detail writer gets
the image through `Icons`, so it joins the render-scoped cache: an Arrow Down
decodes the newly selected preview once, and a keystroke that keeps the
selection decodes nothing. No new component: the list with a pane and the
`Detail` view already exist; no `Theme` colour is added.

### 3. A preview file made at record time

`record` writes `<id>.preview.png`, fitting 800 x 800, beside the image and its
thumbnail, and makes the 384 px thumbnail from the preview rather than the
original, which is cheaper. Same lifecycle as the thumbnail: counted in
`bytes`, deleted with the entry, kept by the orphan sweep. No preview when the
image already fits; the pane then shows the original, which is as cheap.

- **Size, 800 px:** the pane image is at most about 370 logical px wide,
  740 physical at 200 %. 800 stays sharp there.
- **Which file the pane shows:** the preview when it exists; otherwise the
  original when the stored size says it fits in 800 px; otherwise (rows
  recorded before this change, with no stored size) the thumbnail. Old rows
  look softer in the pane but never decode a full image. They roll out of the
  history on their own; alpha needs no backfill.

Alternatives:

- **Show the full image.** The renderer decodes on the main thread with
  `Image::load_from_path`. A 4K screenshot costs tens of milliseconds in
  release and far more in debug, on every selection change, plus about 33 MB of
  pixels per image held in the cache. That is the lag the thumbnail change
  removed.
- **One 800 px file for both list and pane.** The list decodes up to 50 rows on
  open: about 128 MB and a slow first open.
- **Scale the thumbnail up.** Blurry at 200 %, which the spec rules out.

### 4. Stored data

A new migration adds `source TEXT`, `width INTEGER`, and `height INTEGER` to
`local_clipboard_history`. Old rows keep `NULL` and show no "Source" or "Size".
`touch` sets `source` along with `copied_at`, so copying again updates both.

Line and character counts are computed by the listing query from the full
text: `length(text)` (characters, as SQLite counts code points) and the count
of `char(10)` plus one. The listing already reads at most 2000 characters
(`LISTED_CHARS`) for the body, so the pane shows that much and appends "…"
when the text is longer.

Alternatives:

- **Count at record time into columns.** Cheaper to read, but wrong for old
  rows and two more columns to keep right. The history is bounded, so the scan
  is small.
- **Read the image size from the file when listing.** A disk read per image on
  every open.

### 5. Source: exactly one candidate

The watcher asks `candidate_applications` once per change, uses the answer for
the exclusion check as before, and passes it to `record`. `record` stores the
name only when there is exactly one candidate. Windows answers with at most
one, so it always stores a known owner; macOS stores one only when a single
application was frontmost around the copy. One platform-neutral rule, no `cfg`.

Excluded content still leaves no trace: the exclusion check runs first and
nothing is recorded.

Alternative: **store the macOS frontmost application.** The macOS attribution
comment explains why that misattributes after a quick switch; showing a wrong
source is worse than showing none.

### 6. Detected kinds

Checked on the trimmed text, only when it is a single line without spaces and
shorter than `LISTED_CHARS`, in this order:

- **Link:** `url::Url::parse` succeeds with an `http` or `https` scheme. `url`
  is already a dependency.
- **Email address:** the `email_address` crate's `EmailAddress::is_valid`, and
  no `mailto:` prefix.
- **Colour:** `#` followed by 3, 4, 6, or 8 hex digits. Hand-rolled: the
  candidate `csscolorparser` also accepts names such as `red` and functions
  such as `rgb(...)`, which would label ordinary words as colours. Restricting
  it to hex leaves a one-line check.
- **File path:** `Path::is_absolute`. On Windows this needs a drive or UNC
  prefix and a root, so a `::{GUID}` shell identifier does not pass (see the
  AGENTS.md trap). No existence check: it would touch the disk per entry.

Anything else is "Text".

### 7. Time formatting

"Copied" is formatted with `time`, already a dependency, from `copied_at` in
the local offset (`OffsetDateTime::now_local`, falling back to UTC like
`templates`). `time` 0.3.55 no longer refuses the local offset in a
multithreaded process on Unix, so this works on macOS too; the macOS hand check
confirms it. Format: "Today, 14:31", "Yesterday, 14:31", "27 Oct, 14:31", or
"27 Oct 2025, 14:31", 24-hour clock.

Alternative: **`jiff`.** Already in the lock file as a transitive dependency,
with proper time zone support, but `time` does the job and is already a direct
dependency.

### Interface copy

New strings, written with the `ux-writing` skill and checked against
`openspec/specs/interface-copy`: all sentence case, no internals, no sentences.

- Metadata labels: "Source", "Copied", "Type", "Lines", "Characters", "Size".
- Type values: "Text", "Link", "Email address", "Colour", "File path", "Image".
- Copied values: "Today, 14:31", "Yesterday, 14:31", "27 Oct, 14:31",
  "27 Oct 2025, 14:31".
- Size value: "1920 × 1080".
- Accessible label of the pane image: "Image".

Unchanged: the row title "Image", the empty state.

## Risks / Trade-offs

- [The list rows get narrower, so text titles cut off sooner] -> The pane shows
  the full text; the row only needs to tell entries apart.
- [Counting characters scans every text on each open] -> Bounded by the entry
  limit and the size ceiling. Measure in the Windows hand check; move to
  columns if it shows.
- [A path or link check labels something wrongly] -> It is only a label with no
  action attached.
- [Old image rows show the soft thumbnail in the pane] -> Temporary; clearing
  the history removes them.
- [macOS local time or attribution behaves differently than read] -> Open macOS
  hand checks in `tasks.md`.

## Migration Plan

One forward migration adding three nullable columns. No backfill, no rollback
path (alpha).
