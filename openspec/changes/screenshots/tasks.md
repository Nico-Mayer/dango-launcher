# Tasks

## 1. Config and folder resolution

- [x] 1.1 Add `ScreenshotsTable` (`enabled`, `commands`, `folders: Vec<String>`) to `src/config/tables.rs`, the `screenshots` field on `Config`, and `"screenshots"` to `BUILT_IN_TABLES`. Verify: config tests load `[screenshots] folders = ["~/Desktop"]` and an absent table; the generated schema and settings reference list `screenshots.folders` (regenerate with the project's schema test)
- [x] 1.2 Add the pure folder resolver in `src/extensions/screenshots/`: non-empty `folders` wins with `~` expanded, empty or absent falls back to a passed-in default. Verify: unit tests cover "Two folders are merged" (both returned in order), "An empty list means the default", and `~` expansion
- [x] 1.3 macOS: implement `platform::screenshot_folder()` with `NSUserDefaults::persistentDomainForName("com.apple.screencapture")` key `location`, `~` expanded, falling back to `dirs::desktop_dir()`; enable the `objc2-foundation` features it needs. Verify: `cargo build` and clippy pass; an `#[ignore]` test (like the real-clipboard tests) asserts it returns a folder that exists
- [x] 1.4 Windows: implement `platform::screenshot_folder()` with `SHGetKnownFolderPath(&FOLDERID_Screenshots, KF_FLAG_DEFAULT, None)`, freeing the string with `CoTaskMemFree`. Check symbol names against the vendored `windows` 0.62 source. Verify: `cargo build` and clippy pass on Windows (left open until a Windows session)

## 2. Scanning and grid

- [x] 2.1 Add the scanner: `read_dir` per folder, not recursive, keep png/jpg/jpeg/webp ignoring case, skip missing folders, sort by modified time newest first, report whether any folder existed. Verify: unit tests on temp dirs cover "Other files are ignored", "Upper-case extensions count", "A missing folder is skipped", and newest-first order across two folders
- [x] 2.2 Add the extension (`dango.screenshots`, name "Screenshots") with `Search Screenshots`: a `GridView` with 4 columns, aspect 16/10, one tile per screenshot (id = path, title = file name, `TileContent::Image` = path), and both empty states from design.md. Register it in `builtin_manifests` and `src/app/startup.rs`; add any missing icons to `src/slint_shell/icons.rs`. Verify: `every_built_in_manifest_validates` passes; tests cover "Grid opens newest first", "No screenshots yet", "No folder exists", and a root search test finds all three command titles for "screenshot" and none when the extension is disabled
- [x] 2.3 Add Open, Show in Finder / File Explorer, and Copy path actions, calling a new `platform::reveal(&Path)`. Verify: tests check the action list and labels per platform and that Copy path returns `ActionOutcome::CopyToClipboard` with the full path
- [x] 2.3a macOS: implement `platform::reveal` by lifting the body from `platform/macos/apps.rs` and call it from the macOS `AppIndexer::reveal`. Verify: `cargo build`, clippy, and the existing applications tests pass
- [x] 2.3b Windows: implement `platform::reveal` by lifting the `explorer /select,` body from `platform/windows/apps.rs` and call it from the Windows `AppIndexer::reveal`. Verify: `cargo build`, clippy, and the applications tests pass on Windows (left open until a Windows session)
- [x] 2.4 Add a UI test on the Slint testing backend that opens the grid with fixture screenshots and narrows it by typing part of a file name. Verify: `cargo test` covers "Grid narrows by file name"

## 3. Clipboard write of image and file

- [x] 3.1 Add `ClipboardSource::set_image_file(&RustImageData, path)` as a default no-op method, so the existing impls and fakes need no change. Verify: `cargo build` and `cargo test` pass unchanged
- [x] 3.1a macOS: implement it on `CrateClipboard` with one `ClipboardContext::set(vec![Image(img), Other("public.file-url", url)])`, so the PNG and the file URL sit in one pasteboard item (not `Files`, which the crate writes as separate items). Verify: an `#[ignore]` test round-trips a fixture png and jpg through the real clipboard and reads back both an image and the file path
- [x] 3.2 Windows: implement it on `CrateClipboard` with `clipboard-win` directly (new direct Windows dependency) inside one open clipboard: empty, DIB, then `CF_HDROP` with `NoClear`. Not through clipboard-rs `set`, whose nested `set_image` closes the clipboard and drops the files silently. Add this as a platform trap in AGENTS.md. Verify: the round-trip test from 3.1a passes on Windows (left open until a Windows session)
- [x] 3.3 Add `TextTarget::paste_screenshot(&Path)` as a default method returning `TextError::Unavailable`, implemented on `TextExchange` only: read and decode the file before dismissing, then the same steps as `paste_content` but writing with `set_image_file` and without `own_writes.expect`. Verify: tests with the fake clipboard and keys check the order (no dismiss and no write when the file is missing, does not decode, or permission is absent), that the screenshot stays on the clipboard, and that the watcher records it as a newest file entry ("A copied screenshot is recorded", "A pasted screenshot stays on the clipboard")
- [x] 3.3a Add a watcher test for the selection-and-paste scenario "A copied path is recorded": a text write without `expect` becomes the newest entry. Verify: `cargo test`
- [x] 3.4 Add Paste (Enter) and Copy actions to tiles, and the `Paste Latest Screenshot` and `Copy Latest Screenshot` no-view commands. Copy reads and decodes the file, calls `set_image_file`, then returns `ActionOutcome::Copied` or `succeed_with("Copied to clipboard")`. Verify: tests with fakes cover "Copy the latest screenshot", "No screenshot to act on" (both commands, clipboard untouched), and "A screenshot deleted meanwhile" for paste and copy

## 4. Move to trash

- [x] 4.1 Add `trash = "5.3"` and the Move to Trash / Move to Recycle Bin action: on success return `ActionOutcome::Replaced` with a rescanned grid, on failure the platform's failure string and log the error. Put the trash call behind a small function the tests can swap. Verify: tests cover the label per platform, the grid without the item after success, and "The move fails" (item stays, failure text)

## 5. Hand checks

- [x] 5.1 macOS: in a debug build driven with `dango-ui`, open `Search Screenshots` on the author's folder and check thumbnails fill in without lag; paste into a chat app (image appears), copy then paste in Finder (file copy appears) and in Preview's New from Clipboard (image), run `Paste Latest Screenshot` from a bound hotkey, Show in Finder, and Move to Trash then Put Back from Finder
- [ ] 5.2 Windows: take a screenshot with Win+PrtScn, run `Search Screenshots` and check it is first; paste into a chat app (image), copy then paste in File Explorer (file copy) and in Paint (image), run `Paste Latest Screenshot` from a hotkey (`mise run keys`, check foreground with `mise run shot`), Show in File Explorer, and Move to Recycle Bin then restore

## 6. Wrap-up

- [x] 6.1 Add any new platform trap or build fact found in tasks 1 to 5 to AGENTS.md. Verify: `mise run check` and `cargo test` pass
- [x] 6.2 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context; report findings to the author

## Workflow follow-up

- Archive only after the Windows tasks 1.4, 2.3b, 3.2, and 5.2 are done in a Windows session.
