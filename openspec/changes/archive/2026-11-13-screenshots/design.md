# Design

## Context

See proposal.md for why. Behaviour is in `specs/screenshots/spec.md` and the `selection-and-paste` delta.

What already exists and shapes the approach:

- Grid view in the view protocol (`GridView`, `TileContent::Image { image }` takes a file path). `games` is the one built-in grid today.
- The renderer turns a large image path into a thumbnail on a worker thread, caches it under `cache_dir()/thumbnails` keyed by path + modified time, and shows it on the next render (`src/slint_shell/icons.rs`, `is_large`, `thumbnail_key`).
- Clipboard writes go through `ClipboardSource` (`src/extensions/clipboard/source.rs`), implemented on clipboard-rs. Paste goes through `TextTarget::paste_content(&Content)` (`src/text/mod.rs`), which marks the write as Dango's own (`own_writes.expect`) so the history skips it.
- The history reads files before images (`ClipboardWatcher::read` in `src/extensions/clipboard/watcher.rs`), so a write of image + file is recorded as a file entry.
- A Copy action (`Effect::CopyText`, `src/app/mod.rs`) already writes without `own_writes.expect`, so it is recorded today. The spec deltas make that rule explicit instead of adding a screenshot-only exception.
- Reveal in file manager exists only inside `AppIndexer` (`platform/macos/apps.rs` uses `activateFileViewerSelectingURLs`, `platform/windows/apps.rs` runs `explorer /select,`).
- `windows` crate already has `Win32_UI_Shell` on, which holds `SHGetKnownFolderPath` and `FOLDERID_Screenshots`.

## Goals / Non-Goals

**Goals:**

- One built-in Rust extension, `dango.screenshots`, no new shared infrastructure beyond one clipboard write and one reveal function.
- Nothing on the hotkey or startup path: folders are read only when a command runs.

**Non-Goals:**

- Own thumbnail code or cache. The renderer's is reused as is.
- Any change to the view protocol or extension manifest.

## Decisions

### Built-in extension, table `screenshots`

`src/extensions/screenshots/` with the manifest and three commands, registered like `games`. `ScreenshotsTable` in `src/config/tables.rs` with `enabled`, `commands`, and `folders: Vec<String>`; `screenshots` joins `BUILT_IN_TABLES`. Schema and settings reference regenerate from it.

Rejected: a JS extension. It would need a host API for image + file clipboard writes and for trash, both of which would only serve this one extension.

### Folder resolution

`folders` set and not empty: each entry, `~` expanded with `dirs::home_dir`. Otherwise `platform::screenshot_folder() -> Option<PathBuf>`:

- macOS: `NSUserDefaults::persistentDomainForName("com.apple.screencapture")`, key `location`, `~` expanded (the author's value is `~/Pictures/screenshots`). Missing or empty: `dirs::desktop_dir()`.
- Windows: `SHGetKnownFolderPath(&FOLDERID_Screenshots, KF_FLAG_DEFAULT, None)`. Follows a moved or OneDrive-redirected folder.

Rejected:

- `defaults read com.apple.screencapture location` as a child process: spawns a process per command run and parses text, for a value the Foundation API returns directly.
- `dirs` crate: has no screenshots folder on either platform.
- `known-folders` crate: wraps the same `SHGetKnownFolderPath` call; one call through the `windows` crate already in the build is less than a new dependency.
- Spotlight query for `kMDItemIsScreenCapture` on macOS: finds screenshots anywhere, but has no Windows counterpart, so the two platforms would disagree on what a screenshot is.

### Scanning

On each command run, `std::fs::read_dir` per folder, not recursive. Keep files whose extension is png, jpg, jpeg, or webp ignoring case, read `modified()` from the same `DirEntry::metadata`, sort newest first. The scan runs inside the command's invocation, off the UI thread. The grid gets every screenshot as a tile; the launcher filters by title and caps at 50 (`ui::ITEM_LIMIT`), so a name search still reaches older ones. Tile id is the full path.

Rejected: a folder watcher (`notify`). A read per run is cheap, and a watcher is a resident service for a list that is only looked at on demand.

### Grid layout and thumbnails

`GridView { columns: 4, aspect: 16.0 / 10.0, .. }`, tile title the file name, `TileContent::Image { image: <path> }`. Screenshots are above the renderer's large-file threshold, so they go through its thumbnail path: placeholder first, then the shrunk image, cached across runs.

Rejected: making thumbnails in the extension like clipboard history does. The renderer already does it for any image path, off-thread and cached by modified time.

### Image + files in one clipboard write

New `ClipboardSource::set_image_file(png: &[u8], path: &Path) -> Result<(), String>`, a default method that returns an error, so only the crate impl and the test fakes that check it change; the other impls (`text/mod.rs`, `clipboard/extension.rs`, `ai/extension.rs` fakes) are untouched. The crate impl builds a `platform::ImageFile` once, then retries only its `write` while the clipboard is busy.

The extension prepares the PNG bytes (`png_of`) before anything is dismissed or written: a PNG goes through as it is once its header reads (`ImageReader::into_dimensions`, no full decode), jpg and webp are decoded and encoded as PNG with the `image` crate. A missing file fails with "That screenshot no longer exists", anything else unreadable with "Couldn't read the screenshot".

The two platforms need different writes, because clipboard-rs 0.3.5 `set` cannot write both formats correctly on either:

- macOS: `set` writes `Files` on its own through `set_files` as separate pasteboard items, then one more item for the rest (`platform/macos.rs`). Instead call `set(vec![Other("public.png", png), Other("public.file-url", url)])`: every `Other` goes into the same `NSPasteboardItem`, so one item carries both types, and the PNG bytes need no decode. The URL comes from `url::Url::from_file_path`.
- Windows: the image arm of `set` calls `set_image`, which opens a second `clipboard_win::Clipboard`; its drop closes the clipboard, so the `Files` write after it fails and `set` still returns `Ok` (`clipboard-win-5.4.1/src/lib.rs` drop). Write with `clipboard-win` directly inside one open clipboard: empty, registered `PNG`, a 24-bit `CF_DIB` (BMP without its 14-byte file header, written with `set_without_clear`, since `set_bitmap_with` mishandles the V4 header of an RGBA BMP), then `CF_HDROP` with `NoClear`. `clipboard-win` becomes a direct Windows dependency; it is already in the tree through clipboard-rs.

Rejected:

- Image only: Finder, File Explorer, and Slack's attach-on-paste want a file.
- File only: chat inputs and image editors on Windows mostly ignore `CF_HDROP` for pasted images.

### Paste and copy paths

Paste: new `TextTarget::paste_image_file(png: &[u8], path: &Path)`, a default method returning `TextError::Unavailable`, so only `TextExchange` and the fakes that test it implement it; the other impls (`app/tests.rs`, `clipboard/extension.rs`, `snippets/extension.rs`, `snippets/service.rs`, `ai/extension.rs`) are untouched. Same order as `paste_content` (permission, dismiss, yield to the previous app, write, paste key, settle), but it calls `set_image_file` and does not call `own_writes.expect`, so the history records the write as a file entry (selection-and-paste delta). The extension calls `png_of` before it, so a deleted or broken file fails with the launcher still up and the clipboard untouched, and no conversion runs between dismiss and paste. On Windows the bitmap is still made from the PNG inside the write.

Copy: the extension holds the `Arc<dyn ClipboardSource>` and calls `set_image_file` directly, then returns `ActionOutcome::Copied` (grid) or `ctx.succeed_with("Copied to clipboard")` (no-view command).

Rejected: a new `Content` variant routed through `paste_content`. `Content` is the clipboard history's stored entry type; a screenshot is never stored as one, and every `match` on `Content` in the history would grow an arm it never uses.

### Reveal and open

Lift the two reveal bodies from `platform/*/apps.rs` into `platform::reveal(path: &Path) -> io::Result<()>`, used by both `AppIndexer::reveal` impls and the new extension. Open goes through the existing opener (`src/opener.rs`). Copy path returns `ActionOutcome::CopyToClipboard(path)`.

### Move to trash

`trash` 5.3 (`trash::delete`): `IFileOperation` on Windows, Finder on macOS. On success the action returns `ActionOutcome::Replaced` with a freshly scanned grid, like clipboard history's delete.

On macOS `trash`'s default method asks Finder over Apple Events, which keeps Finder's "Put Back". Dango already holds the Apple Events entitlement and usage string for Empty Trash, and the user already granted Automation for Finder.

Rejected:

- `trash`'s `NsFileManager` method on macOS: needs no Automation grant, but the item cannot be put back from Finder.
- Hand-rolled `IFileOperation` and `NSFileManager trashItemAtURL`: the crate already wraps both.

### Versioned contracts

No change to the extension manifest or the view protocol. The extension uses existing grid, action, and outcome types only.

### Design-system components

Grid view (`ui/views/` grid), its tiles with image content, the SearchBar, the action panel, the empty view, and the failure ticker. No new component.

### User-visible strings

Written against `openspec/specs/interface-copy`.

| Where | String |
|---|---|
| Extension name | "Screenshots" |
| Command title | "Search Screenshots" |
| Command title | "Paste Latest Screenshot" |
| Command title | "Copy Latest Screenshot" |
| Action (Enter) | "Paste" |
| Action | "Copy" |
| Action | "Open" |
| Action, macOS | "Show in Finder" |
| Action, Windows | "Show in File Explorer" |
| Action | "Copy path" |
| Action, macOS | "Move to Trash" |
| Action, Windows | "Move to Recycle Bin" |
| Empty state title | "No screenshots yet" |
| Empty state text | "Screenshots you save will show up here" |
| Empty state title, no folder | "Screenshot folder not found" |
| Empty state text, no folder | "Check the screenshot folders in the config file" |
| Success notice | "Copied to clipboard" (existing) |
| Failure | "That screenshot no longer exists" |
| Failure | "No screenshots found in your screenshot folders" |
| Failure, macOS | "Couldn't move the screenshot to the Trash" |
| Failure, Windows | "Couldn't move the screenshot to the Recycle Bin" |
| Failure, open | "Couldn't open the screenshot" |
| Failure, unreadable file | "Couldn't read the screenshot" |
| Failure, reveal macOS | "Couldn't show the screenshot in Finder" |
| Failure, reveal Windows | "Couldn't show the screenshot in File Explorer" |

Keywords for root search: "screenshot", "screen", "capture", "image", "paste", "copy".

## Risks / Trade-offs

- [macOS app that sees a file URL pastes the file icon instead of the image] → hand check in a chat app and Preview. Both types sit in one pasteboard item, so there is no item order to tune; if an app picks wrong, note it as a limit.
- [Windows chat app that prefers `CF_HDROP` attaches a file instead of an inline image] → hand check; acceptable, the screenshot still arrives.
- [Recorded screenshot entry names the paste target as its "Source", since macOS attribution looks at the frontmost apps around the change] → accept; hand check and note it.
- [Windows Screenshots folder may not exist until the first Win+PrtScn, so a fresh setup shows "Screenshot folder not found"] → accept; the hand check on Windows confirms which.
- [A folder with thousands of screenshots makes each run slow to scan] → one `read_dir` plus metadata per entry is fast on a local disk; filtering and the 50-tile cap keep the UI bounded. Revisit only if it shows.
- [History records a pasted screenshot as a file entry pointing at a file the user may later trash] → the history already handles file entries whose files are gone.
- [Screenshots under the large-file threshold load at full size] → those are small by definition.
- [Finder Automation prompt on first trash, on a machine that never ran Empty Trash] → one prompt, same as Empty Trash.
