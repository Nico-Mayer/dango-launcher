## 1. Parser

- [x] 1.1 Add `f13` to `f20` to `KEY_TOKENS` in `src/config/hotkey.rs`. Verify: `every_token_in_the_tables_parses` passes under `cargo test`.
- [x] 1.2 Add `HotkeyError::NoModifier` and return it from `parse_with_hyper` when the modifier set is empty and the key is not F1 to F20. Verify: new `cargo test` cases in `hotkey.rs` - `a`, `space`, `k` refused with `NoModifier`; `f1`, `f13`, `f20` accepted with no modifiers; `f21` refused as `UnknownKey`; `win` and `cmd` refused as `NoKey`.
- [x] 1.3 Add a test in `src/hotkeys.rs` that a launcher hotkey of `space` falls back to the default with a problem line, and one that a command bound to `k` yields a conflict line naming it while other bindings stay. Verify: `cargo test hotkeys`.

## 2. Schema and reference

- [x] 2.1 Extend `hotkey_pattern` in `src/config/schema.rs` with a lone function-key alternative built from `KEY_TOKENS`, and update the `hotkey_section` text to the line in design.md. Verify: `every_token_matches_the_hotkey_pattern` extended to check lone `f1`..`f20` match and `a`, `space`, `f21`, `ctrl+f24` do not, under `cargo test`.
- [x] 2.2 Add a test that walks every key token alone and asserts the parser and the pattern agree on it. Verify: `cargo test schema`.
- [x] 2.3 Regenerate `docs/config.md` with `env DANGO_UPDATE_DOCS=1 cargo test the_settings_reference_is_current`. Verify: `the_settings_reference_is_current` passes without the variable.

## 3. Hand checks

- [x] 3.1 Windows: set `launcher.hotkey` to `f13` in a scratch `DANGO_CONFIG_DIR`, run a debug build, press F13 with `scripts/send-keys.ps1`, confirm the launcher shows (`scripts/screenshot.ps1`). Then set it to `space`, confirm the tray notice and that Alt+Space summons the launcher. Done 2026-11-01: F13 toggles the launcher; `space` is refused with the log line "the hotkey has no modifier; only F1 to F20 can stand alone" and Alt+Space works. The tray notice itself was not viewed.
- [x] 3.2 macOS: same as 3.1 with `f13` and `space`, Option+Space as the default. Also bind `f18` with the hyperkey on and check whether it fires (see design, Risks); decide refuse or document. Leave open until a session on the Mac. Checked 2026-11-01 with synthetic key events: F13 toggles the launcher; `space` is refused with the same log line as on Windows and Option+Space works (tray notice not viewed). `f18` with the hyperkey on never fires and shows no notice; with the hyperkey off it fires. Decided: document. The hyperkey setting's description and the global-activation delta state the clash.

## 4. Review

- [x] 4.1 Run `hk check` and `cargo test`.
- [x] 4.2 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context. It reports findings only; the author decides what to fix.
