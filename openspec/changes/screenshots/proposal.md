# Proposal

## Why

Getting a recent screenshot into a chat, a ticket, or an editor takes a trip to Finder or File Explorer, a hunt through a folder, and a drag. Dango can find screenshots on its own and paste or copy them from the keyboard, the way Raycast does.

## What Changes

- New built-in `screenshots` extension with three commands:
  - `Search Screenshots` opens a grid of screenshots, newest first, narrowed by file name as the user types. Enter pastes the selected one into the previous app. The action panel offers Copy, Open, Show in Finder / File Explorer, Copy path, and Move to Trash / Recycle Bin.
  - `Paste Latest Screenshot` and `Copy Latest Screenshot` are no-view commands that act on the newest screenshot.
- Copy and paste put the screenshot on the clipboard as image and as file in one write, so each app takes the format it understands. The screenshot stays on the clipboard after a paste and is recorded in the clipboard history as a file entry.
- Screenshots are the png, jpg, jpeg, and webp files in the screenshot folders. "Newest" is by modified time across all folders.
- Default folder: on macOS the location set for the system screenshot tool, else the Desktop; on Windows the Screenshots known folder (where Win+PrtScn and Snipping Tool save). A new `[screenshots] folders` setting replaces the default with a list of folders.
- Platforms: Windows and macOS.

## Non-goals

- Screen recordings, HEIC, or any other file type. HEIC would need a platform decoder for thumbnails.
- Taking screenshots, or editing, annotating, renaming, or uploading them.
- Watching folders for new files. The folders are read each time a command runs.
- Finding screenshots outside the configured folders (Spotlight `kMDItemIsScreenCapture` on macOS has no Windows counterpart).
- A setting for what Enter does. Enter always pastes.
- A detail pane or metadata view for a screenshot.
- Linux (out of scope for the project).

## Capabilities

### New Capabilities

- `screenshots`: the `screenshots` extension's commands, which files count as screenshots and where they are looked for, what each action does, and the `[screenshots]` setting.

### Modified Capabilities

- `selection-and-paste`: a pasted screenshot stays on the clipboard like a pasted history entry. A copy the user asks for (a Copy action, or a screenshot copied or pasted) is recorded in the clipboard history, as Copy actions already are today.
- `clipboard-history`: the "Dango's own writes are not recorded" rule points at the exception above.

## Impact

- New `src/extensions/screenshots/`, registered in `src/extensions/mod.rs` and `src/app/startup.rs`.
- New `ScreenshotsTable` in `src/config/`, added to `BUILT_IN_TABLES`; schema and settings reference regenerate.
- Clipboard write of image plus file in one go: new default method on `ClipboardSource` (`src/extensions/clipboard/source.rs`) and a paste path in `src/text/` that does not mark the write as Dango's own. macOS goes through clipboard-rs; Windows writes directly with `clipboard-win`.
- Platform: default screenshot folder per OS behind a narrow function in `src/platform/`; reveal-in-file-manager reused or lifted from `platform/*/apps.rs`.
- No new thumbnail code: the renderer already shrinks large image files off the UI thread and caches them on disk (`src/slint_shell/icons.rs`).
- New dependencies: `trash` 5.3 for Move to Trash / Recycle Bin; `clipboard-win` 5.4 as a direct Windows-only dependency (already in the tree through clipboard-rs).
