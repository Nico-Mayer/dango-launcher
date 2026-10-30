## 1. Permission state

- [x] 1.1 Add `Permission`, the `Permissions` trait, and `settings_url` to `src-tauri/src/platform/mod.rs`, with a fake for tests; verify with `cargo build` from `src-tauri`
- [x] 1.2 macOS: implement `Permissions` in `src-tauri/src/platform/macos/` with `AXIsProcessTrusted`, `CGPreflightListenEventAccess`, the existing Accessibility prompt, and `CGRequestListenEventAccess`; verify with `cargo clippy -- -D warnings` and, in a debug run on the author's Mac, that `missing()` returns `InputMonitoring`
- [x] 1.3 Windows: implement `Permissions` with an empty `missing()` and a no-op `request()`; verify that CI on `windows-latest` builds and passes clippy

## 2. Pinned candidates

- [x] 2.1 Add `pinned: bool` to `Candidate` in `src-tauri/src/search/mod.rs`, default false everywhere it is built; verify with `cargo build`
- [x] 2.2 In `src-tauri/src/ranking/mod.rs`, put pinned candidates first on an empty query in provider order, outside `SUGGESTION_LIMIT`, and ignore `pinned` on a typed query; verify with ranker tests for both cases via `cargo test ranking`
- [x] 2.3 Carry `pinned` to the frontend `ResultItem` and render pinned results under a "Needs attention" heading above Suggestions in `src/App.svelte`; verify with `npm run check`

## 3. Permissions extension

- [x] 3.1 Add `triangle-alert` to the icon registry in `src/lib/Icon.svelte`; verify with `npm run check`
- [x] 3.2 Add a built-in `permissions` extension under `src-tauri/src/extensions/permissions/` with a root provider that returns one pinned warning item per missing permission, titles "Accessibility is off" and "Input Monitoring is off", subtitles naming the features, keywords with the permission name and "permission", Accessibility first; verify with unit tests against the fake `Permissions` for none, one, and both missing via `cargo test permissions`
- [x] 3.3 Give each item the primary action "Grant access", which calls `request`, opens `settings_url` through the opener, and hides the launcher; verify with a unit test that the fake records the request and the outcome hides the launcher
- [x] 3.4 Register the extension in `src-tauri/src/lib.rs` and regenerate the config schema and `docs/config.md`; verify with `cargo test` (the stale-reference test passes)

## 4. Poller and tray

- [x] 4.1 (dropped: macOS applies an Input Monitoring grant only after a restart, so `ExtensionHost::restart_services` was added, then removed)
- [x] 4.2 Move the startup notices into app state and build the tray menu from a function that takes the notices, the missing permissions, and the config status; add the `grant-accessibility` and `grant-input-monitoring` menu events; verify with `cargo build` and a debug run on macOS showing the grant item
- [x] 4.3 On macOS, start a thread that asks `missing()` every 2 seconds and, on a change, rebuilds the tray menu on the main thread; verify in a run on macOS that granting a permission removes its tray item within 2 seconds
- [x] 4.4 (dropped with 4.1: the hyperkey and keyword expansion start on the restart macOS requires, through the normal startup path)
- [x] 4.5 Make the hyperkey failure in `apply_hyperkey` and the `MacKeyMonitor::start` log name Input Monitoring when it is missing; verify by reading `dango.log` after a start without the permission

## 5. Verification

- [x] 5.1 macOS, bundled debug app started with `open`, both permissions reset with `tccutil`: open the launcher and verify both warning items are on top, Accessibility first; type "permission" and verify both are found; verify both tray items
- [x] 5.2 macOS, same run, hyperkey configured and snippets enabled: confirm "Grant access" on Input Monitoring, verify System Settings opens at Input Monitoring with Dango listed; grant it, let macOS reopen Dango, and verify the hyperkey works
- [x] 5.3 macOS, same run: grant Accessibility through its warning and verify the pane, the item gone on the next open, and a snippet paste working
- [x] 5.4 macOS: run with both permissions granted and `DANGO_MEASURE=1`, verify no warning anywhere and activation timing unchanged against a run of the previous build
- [x] 5.5 macOS: revoke Input Monitoring while Dango runs and verify the tray item and the warning item come back
- [x] 5.6 Windows, installed build: open the launcher and the tray, verify no permission warning, and that the hyperkey and keyword expansion still work

## 6. Wrap up

- [x] 6.1 Review the diff for comments that restate the code and for scope beyond the proposal; verify `cargo test` and `cargo clippy -- -D warnings` pass from `src-tauri`
- [x] 6.2 Record in the known platform traps in `openspec/config.yaml` that keyboard event taps need Input Monitoring, and whether the hyperkey's active tap also needs Accessibility; verify the entry reads as short bullets beside the existing macOS traps
