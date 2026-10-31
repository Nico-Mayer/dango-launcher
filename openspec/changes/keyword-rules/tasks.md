## 1. Overlapping keywords

- [x] 1.1 Add `RecordError::KeywordOverlaps` with the message "One keyword can't be the start of another, so choose a different keyword", and refuse an overlap in `validate_keyword` in `src/extensions/snippets/store.rs` after the exact-duplicate check, case-sensitive, skipping the record being edited; verify with new tests in `store.rs` for `;s` then `;sig`, `;sig` then `;s`, `;s` with `;Sig` allowed, and an edit that lengthens its own keyword, plus the new variant in `every_fixed_message_is_a_sentence` (`cargo test snippets`)

## 2. Skip keystrokes aimed at Dango on Windows

- [x] 2.1 Add `platform::foreground_is_dango()` with a Windows implementation in `src/platform/windows/mod.rs` that compares the foreground window's process id with `std::process::id()`, and a macOS implementation that returns false; verify `cargo clippy --all-targets -- -D warnings` passes
- [x] 2.2 In `run` in `src/extensions/snippets/service.rs`, clear the buffer and skip the character when `foreground_is_dango()` is true, before the exclusion check, with the decision in a pure helper next to `is_excluded`; verify with a new test in `service.rs` (`cargo test snippets`)
- [ ] 2.3 Hand check on Windows: start a debug build, open the launcher with `scripts/send-keys.ps1`, type a snippet's keyword, and confirm with `scripts/screenshot.ps1` that the search field shows the keyword unexpanded; then type the keyword in Notepad and confirm it expands

## 3. Skip keystrokes aimed at Dango on macOS

- [ ] 3.1 In `translate` in `src/platform/macos/keymonitor.rs`, return `Clear` when the event's `EventTargetUnixProcessID` equals Dango's own process id, with the comparison in a pure function tested next to `characters`; verify `cargo test` on a Mac
- [ ] 3.2 Hand check on macOS: type a snippet's keyword into the launcher over another app and confirm it stays unexpanded, then type it in TextEdit and confirm it expands. If the target process id is 0 at the tap, stop and update design.md (see Risks) before going on

## 4. AltGr characters on Windows

- [ ] 4.1 Move the chord test in `src/platform/windows/keymonitor.rs` into a pure function that, with right Alt down and no Win key, answers "translate with Ctrl+Alt" instead of `Clear`; set Ctrl and Alt in the `ToUnicodeEx` state array in that case; verify with a new test of the pure function covering AltGr, left Ctrl+Alt, Ctrl alone, and Win (`cargo test keymonitor`)
- [ ] 4.2 Add the trap to AGENTS.md "Platform traps": Windows reports AltGr as left Ctrl plus right Alt, so a chord check must look at `VK_RMENU` before treating Ctrl+Alt as a shortcut; verify the entry is there
- [ ] 4.3 Hand check on Windows with the German layout: give a snippet the keyword `@@mail`, type it in Notepad, and confirm it expands; confirm Ctrl+C still clears a half-typed keyword

## 5. Wrap up

- [ ] 5.1 Run `hk check` and `cargo test`; verify both pass
- [ ] 5.2 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context; it reports findings only and the author decides what to fix
