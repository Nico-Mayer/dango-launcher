## 1. Windows path classification

- [x] 1.1 Replace the body of `is_file_path` in `src-tauri/src/platform/windows/apps.rs` with a check that the first `Path` component is a `Component::Prefix`; extend `recognises_windows_file_paths` with `::{52205FD8-5DFB-447D-801A-D0B52F2E83E1}` expected false and keep the existing cases; verify with `cargo test is_file_path` from `src-tauri` on Windows
- [x] 1.2 In `enumerate`, store `target` only when `is_file_path` accepts it, and drop the now redundant shape checks from `launch` and `reveal`; verify with `cargo clippy -- -D warnings` and, in a dev run, that the Explorer result shows no subtitle and offers only Open

## 2. Core launch path

- [x] 2.1 In `src-tauri/src/extensions/applications/index.rs`, make `rebuild` keep the current index when the indexer returns an empty list and the index is not empty, logging the skip; verify with a unit test using `FakeIndexer::set_apps(&[])` after a populated rebuild
- [x] 2.2 In `src-tauri/src/extensions/applications/mod.rs`, change `perform` so any launch error rebuilds the index and then reports "no longer installed" only when the id is absent, otherwise the "Couldn't open" message; delete `AppIndex::remove`, its test, and `Store::delete_app` if nothing else calls them; verify with `cargo build` and `cargo clippy -- -D warnings` from `src-tauri`
- [x] 2.3 Make the `get` miss at the top of `perform` rebuild once and retry before reporting the application as gone; verify with `cargo build`
- [x] 2.4 Add extension tests with `FakeIndexer`: a `NotFound` launch while the fake still lists the app returns the "Couldn't open" message and keeps the entry; a `NotFound` launch after `set_apps` dropped it returns the "no longer installed" message and the entry is gone; a `get` miss for an app the fake lists ends in `Done`; verify with `cargo test applications` from `src-tauri`

## 3. Verification

- [x] 3.1 Windows, dev run: type `explorer`, verify the result shows no subtitle and only the Open action, press Enter, verify File Explorer opens, the result stays in root search, and `local_app_index` in `%APPDATA%\com.nimayer.dango\dango.sqlite` has the `Microsoft.Windows.Explorer` row afterwards
- [x] 3.2 Windows: create a Start Menu shortcut to a copy of a small executable, wait for it to appear in results, delete the shortcut and the copy, launch it, and verify the "no longer installed" message and that it is gone from results
- [ ] 3.3 (deferred, no Mac at hand; verify on the next macOS session) macOS, dev run: launch an application from root search and verify it opens; copy a small `.app` into `~/Applications`, wait for it to appear, delete it, launch it, and verify the "no longer installed" message and that it is gone from results

## 4. Wrap up

- [x] 4.1 Review the diff for comments that restate the code and for scope beyond the proposal; verify `cargo test` and `cargo clippy -- -D warnings` pass from `src-tauri`
- [x] 4.2 Add the `::{GUID}` shell namespace target to the known platform traps in `openspec/config.yaml`; verify the entry reads as one short bullet beside the existing Windows traps
