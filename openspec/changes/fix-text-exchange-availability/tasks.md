## 1. Truthful error

- [x] 1.1 Add `TextError::Unavailable` in `src-tauri/src/text/mod.rs` with the message "Dango can't reach other apps right now. See dango.log for the reason."; verify with `cargo build` from `src-tauri`
- [x] 1.2 In `ai/extension.rs`, `clipboard/extension.rs`, and `snippets/extension.rs`, map a missing text target to `Unavailable` instead of `PermissionMissing`, and update the tests that assert the old message for a `None` target (for example `pasting_without_key_injection_fails_the_same_way`); verify with `cargo test` from `src-tauri`

## 2. Exchange without the database

- [x] 2.1 In `src-tauri/src/lib.rs`, open `CrateClipboard` and build the text exchange before the store check, with the history `Watcher` as `OwnWrites` when the store exists and `Unwatched` otherwise; the clipboard extension still registers only with both; verify with `cargo build` and a dev run on macOS where pasting a snippet still works
- [x] 2.2 Push "Pasting is off, see dango.log" to the startup notices when the exchange is `None`; verify with `cargo build` and by reading the notice list in setup

## 3. Lazy key injection on macOS

- [x] 3.1 In `src-tauri/src/platform/macos/text.rs`, make `MacKeys::new` infallible and hold `Arc<Mutex<Option<Enigo>>>`; in `on_main`, create the `Enigo` on first use on the main thread, mapping `NewConError::NoPermission` to `PermissionMissing` and other errors to `TargetUnavailable(KEYSTROKE_FAILED)` with the cause logged; verify with `cargo build` and `cargo clippy -- -D warnings` from `src-tauri`
- [x] 3.2 Drop the `?` on `MacKeys::new` in `platform::text_exchange` so the macOS branch always returns an exchange; verify with `cargo build`

## 4. Windows key injection

- [ ] 4.1 Leave `WindowsKeys::new` eager and confirm its failure reaches the tray notice from 2.2; verify that CI on `windows-latest` builds and passes clippy

## 5. Verification

- [ ] 5.1 macOS, dev run, Accessibility revoked for the dev binary: run an AI command with paste output and verify the permission message; grant the permission in System Settings without restarting, run it again, and verify the answer is pasted
- [ ] 5.2 macOS, dev run, permission granted: paste a clipboard history entry and a snippet right after launch and verify both arrive with no visible delay
- [ ] 5.3 macOS, dev run with the database file made unreadable: verify the tray shows the database notice, clipboard history is off, and a snippet and an AI paste still work
- [ ] 5.4 Windows, installed build: run the AI command that showed the Accessibility message before and verify it now pastes, or, if the exchange still fails, that the message names `dango.log`, the tray shows "Pasting is off, see dango.log", and `dango.log` names the cause
- [ ] 5.5 Windows, installed build: paste a clipboard history entry and a snippet and verify both still work

## 6. Wrap up

- [x] 6.1 Review the diff for comments that restate the code and for scope beyond the proposal; verify `cargo test` and `cargo clippy -- -D warnings` pass from `src-tauri`
- [x] 6.2 Add the enigo `NoPermission` behaviour on macOS to the known platform traps in `openspec/config.yaml`; verify the entry reads as one short bullet beside the existing macOS traps
