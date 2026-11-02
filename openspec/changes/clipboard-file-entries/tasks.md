# Tasks

## 1. Files in the history store

- [x] 1.1 Add `Content::Files` and `Kind::Files` (`files` in the database), store paths newline-joined in `text`, fingerprint over `files\0` plus paths, count `bytes` as the joined length; verify with `cargo test` history tests: record two paths, read back as one entry; same paths again reorder, not duplicate
- [x] 1.2 Keep every path of a file entry in `entries()` instead of cutting to `LISTED_CHARS`; verify with a `cargo test` history test of a file entry longer than `LISTED_CHARS`
- [x] 1.3 `History::content` for a file entry keeps only existing paths; none left removes the entry and returns `HistoryError::FilesGone { count }` with "The copied file no longer exists" / "The copied files no longer exist"; verify with `cargo test` history tests for some missing and all missing, and that removing a file entry leaves the files on disk

## 2. Reading and writing files on the clipboard

- [x] 2.1 Add `files()` and `set_files()` to `ClipboardSource`, implemented on `CrateClipboard` through `until_free`, and on every fake source; verify `cargo test` and an `#[ignore]` round-trip test in `source.rs` run on the Mac with `cargo test -- --ignored --test-threads=1`
- [x] 2.2 `Watcher::read` reads files before text and image; `Watcher::restore` writes files; verify with `cargo test` watcher tests: files plus names as text record one file entry, own file write is not recorded, concealed marker and excluded application skip files
- [x] 2.3 `Snapshot::content` returns `Content::Files` ahead of text; `TextExchange::write` writes files; verify with `cargo test` in `src/text/`: pasting a file entry writes files, restoring a clipboard that held files after an insertion records nothing

## 3. Listing and the pane

- [x] 3.1 Add `file` (`lu::LuFile`) to `src/slint_shell/icons.rs`; verify with the registry's `cargo test`
- [x] 3.2 File entry list item: title from first file name plus " and N more", keywords of every file name, icon from `platform::icon_for` through an `IconCache` held by the clipboard extension, `icon:file` when none; verify with `cargo test` extension tests for one file, several files, and a missing path
- [x] 3.3 File entry pane: paths one per line, "Type" "File" or "Files", "Count" for several; empty state description "Text, images, and files you copy will show up here"; strings checked with the `ux-writing` skill against `interface-copy`; verify with `cargo test` extension tests
- [x] 3.4 Paste and copy of a file entry with all files gone show the failure, remove the entry, leave the clipboard unchanged; verify with `cargo test` extension test `a_file_entry_whose_files_have_gone_is_reported_and_dropped` (a failed action keeping the launcher open is the state machine's, already covered)

## 4. macOS hand checks

- [ ] 4.1 Release build with `DANGO_CONFIG_DIR` scratch dir, Accessibility granted: copy two files in Finder, history shows one entry "<name> and 1 more" with the Finder icon and both paths in the pane, no text entry of names; inspect with `dango-ui` `snapshot`
- [ ] 4.2 Paste the entry into a Finder window: Finder copies both files there; copy action then Cmd+V in Finder does the same; no new history entry appears after either
- [ ] 4.3 Delete one of the two files, paste the entry: the other is pasted; delete both, paste: toast "The copied files no longer exist", entry gone
- [ ] 4.4 With files copied in Finder on the clipboard, insert text through Dango (a snippet) into an editor: afterwards the clipboard holds the files again and no new history entry appears
- [x] 4.5 `mise run check` and `cargo test` pass on the Mac

## 5. Windows hand checks

- [x] 5.1 `cargo test` and `mise run check` pass on Windows
- [x] 5.2 Release build with `DANGO_CONFIG_DIR` scratch dir: copy two files in File Explorer, history shows one entry with the Explorer icon and both paths; check with `dango-ui` `snapshot`
- [ ] 5.3 Paste the entry into a File Explorer window: Explorer copies both files; copy action then Ctrl+V does the same; no new history entry; use `mise run keys` and `mise run shot` for what happens outside the launcher
- [ ] 5.4 Missing files on Windows: one gone pastes the rest; all gone shows "The copied files no longer exist" and removes the entry

## 6. Review

- [x] 6.1 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context; report findings to the author

## Workflow follow-up

- Update issue #9 to describe the real scope: file entries were missing on both platforms, not only macOS.
- Archive only after group 5 is done on Windows.
