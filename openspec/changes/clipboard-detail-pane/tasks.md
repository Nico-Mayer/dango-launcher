## 1. Detail can show an image and plain text

- [x] 1.1 Add `image: Option<String>` and `text: Option<String>` to `ItemDetail` and `DetailView` in `src/protocol.rs`, and carry both in `DetailBody` in `src/ui/snapshot.rs`; verify with a protocol test that a detail without them still deserializes, and a snapshot test that a pane carries both (`cargo test protocol snapshot`)
- [x] 1.2 Add `Theme.detail-image-height` (200 px) and extend `ui/views/detail.slint` and `DetailData` in `ui/globals.slint`: a framed `Image` above the body (contain, never enlarged, accessible label "Image") and a `Theme.mono` `Text` for a plain body in place of the `StyledText`; verify `cargo clippy --all-targets -- -D warnings` compiles the `.slint` files
- [x] 1.3 Have the detail writer in `src/slint_shell/render/detail.rs` load the image through `Icons` so it joins the render-scoped cache, and write `plain` and the plain text; verify with UI tests in `src/slint_shell/tests.rs` that a detail with plain text `# not a heading` shows it as written in the monospace face, that an image detail shows the image above the body, that a missing image path still shows body and metadata, and that a keystroke keeping the selection loads no new image (`cargo test`)

## 2. Record source and image size

- [x] 2.1 Add migration `0008` with nullable `source`, `width`, and `height` columns on `local_clipboard_history`; verify with a store test that the migration applies to a database at `0007` (`cargo test store`)
- [x] 2.2 Ask `candidate_applications` once per change in `Watcher::handle_change`, use the answer for the exclusion check, and pass the single candidate (or none) to `History::record`; verify with watcher tests that one candidate is stored, several store none, and excluded content still records nothing (`cargo test clipboard`)
- [x] 2.3 Store `width` and `height` in `record`, and make `touch` update `source` with `copied_at`; verify with history tests that a re-copy from another application updates source and time, and an image row has its size (`cargo test clipboard`)

## 3. Preview file

- [x] 3.1 Write `<id>.preview.png` fitting 800 x 800 in `record` when the image is larger, make the thumbnail from it, count it in `bytes`, delete it in `remove`, `trim`, and the "file has gone" path, and keep it in the orphan sweep; verify with history tests for a large image (preview written, counted, deleted with the entry), a small one (no preview), and the sweep (`cargo test clipboard`)
- [x] 3.2 Add the pane image choice to `Entry` (preview, else the original when the stored size fits, else the thumbnail); verify with history tests for each of the three cases, including a row with no stored size (`cargo test clipboard`)

## 4. History as a list with a detail pane

- [x] 4.1 Read line and character counts in the listing query in `History::entries`; verify with history tests for one line, three lines, and text longer than `LISTED_CHARS` counting in full (`cargo test clipboard`)
- [x] 4.2 Add kind detection (Link via `url`, Email address via the `email_address` crate, hex Colour, File path via `Path::is_absolute`, else Text) as a pure function in `src/extensions/clipboard/`; verify with unit tests for each kind, a sentence containing a link, the word `red`, and a `::{GUID}` identifier (`cargo test clipboard`)
- [x] 4.3 Add "Copied" formatting with `time` as a pure function of the entry time, now, and the local offset; verify with unit tests for today, yesterday, earlier this year, and an earlier year (`cargo test clipboard`)
- [x] 4.4 Set `detail_pane: true` in `history_view` and give each item a detail: plain text (with "…" when cut) or the pane image, and the metadata pairs in the spec's order with the strings from design.md; verify with extension tests for a text entry, an image entry, an entry without source, and a markup text staying plain, plus a UI test that Arrow Down changes the pane (`cargo test`)

## 5. Windows hand checks

- [ ] 5.1 Hand check on Windows in a debug build with the `dango-ui` MCP server and `scripts/send-keys.ps1` / `scripts/screenshot.ps1`: copy text from VS Code and Notepad, an indented code block, a link, and two full-screen screenshots; open Clipboard History and confirm the pane shows each entry as copied, the source names, "Today" times, kinds, counts, and image sizes, and that the screenshots are sharp at the display's scaling
- [ ] 5.2 Hand check on Windows in a release build with `DANGO_CONFIG_DIR` set to a scratch directory: hold Arrow Down through a history full of screenshots and confirm the selection and the pane follow without visible lag, and that memory drops back after leaving the history

## 6. macOS (open until a session on the Mac)

- [ ] 6.1 Verify `cargo test` and `cargo clippy --all-targets -- -D warnings` pass on a Mac
- [ ] 6.2 Hand check on macOS: copy text in one app and confirm "Source" names it; copy, then switch apps at once, and confirm no "Source" is shown; confirm "Copied" shows the local time, not UTC
- [ ] 6.3 Hand check on macOS on a Retina display: confirm a screenshot in the pane is sharp and that moving through screenshots shows no visible lag

## 7. Wrap up

- [x] 7.1 Run `hk check` and `cargo test`; verify both pass
- [x] 7.2 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context; it reports findings only and the author decides what to fix
