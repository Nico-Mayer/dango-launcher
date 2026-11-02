# Tasks

## 1. Drop the exclusion list

- [x] 1.1 Remove `config::Selection`, `RefusesFallback`, `refuses_fallback`, `TextError::SelectionRefused`, the `refuses` argument of `platform::text_exchange`, and their tests; verify `cargo test` passes and an old config with a `selection` key still parses (config test)
- [x] 1.2 Remove the `selection` example from `src/config/schema.rs`, its row from `docs/config.md`, and the exclusion step from `examples/walkthrough/macos.rs`; verify `mise run check` passes

## 2. Lend the foreground in the read path (shared)

- [x] 2.1 Add a lend step to `Handoff` that returns a guard giving the foreground back on drop, and make `TextExchange::selection` use it instead of `Launcher::dismiss`; drop `DirectSelection::needs_foreground`; verify with `cargo test` fakes that a read never dismisses the launcher, the lend is taken before the direct read, and returned on every exit (text, empty, copy fallback, copy error, lend error)
- [ ] 2.2 Show a failed read in the launcher at once: verify with a UI test on the Slint testing backend that running an AI command with no selection leaves the launcher visible showing "Select some text first"

## 3. Windows

- [x] 3.1 Implement the Windows lend: force the previous window forward and wait (shared with `yield_to_previous`), and on drop `force_foreground` the launcher's hwnd; verify `cargo test` and `mise run check`
- [x] 3.2 Add the `LENDING` flag and make the foreground event handler skip the hide while lending or when Dango is the foreground at handling time; verify with a unit test of the decision function
- [ ] 3.3 Send Ctrl+Insert for `WindowsKeys::copy`, with `CHORD_HOLD`; verify `cargo test` and `mise run check`
- [ ] 3.4 Hand check on Windows with `dango-ui` and `scripts/send-keys.ps1`: select text in Notepad, a browser, Windows Terminal and Zed, run Improve Writing from root search and from a command hotkey; the launcher stays on screen, the selection is read, typing goes to the launcher after, the clipboard is unchanged
- [ ] 3.5 Hand check on Windows: with nothing selected in Zed and in Herdr, run Improve Writing; the launcher stays and shows "Select some text first", Windows Terminal running a program is not interrupted by the copy

## 4. macOS

- [ ] 4.1 In `MacSelection::read`, skip focused elements whose pid is Dango's own, and treat `""` as Empty only when the element has `AXSelectedTextRange`; verify `cargo test` and a macOS compile in CI (`mise run check` on the Mac)
- [ ] 4.2 Post `MacKeys::copy`'s Cmd+C to the frontmost application's pid with `CGEvent::post_to_pid`; verify `cargo test` and a macOS compile
- [ ] 4.3 Hand check on macOS with `dango-ui`: select text in TextEdit, a browser and Zed, run Improve Writing from root search and from a command hotkey; the launcher stays key and on screen, the selection is read, the clipboard is unchanged. If the posted copy is ignored, record which app and decide the fallback in design.md
- [ ] 4.4 Hand check on macOS: with nothing selected in TextEdit, run Improve Writing; the launcher shows "Select some text first" and no Cmd+C was needed

## 5. Review

- [ ] 5.1 Run the `ponytail-review` skill in a fresh subagent on the change's commits, without the apply context; record findings for the author

## Workflow follow-up

- Remove `selection.excluded-applications` from the author's config.
- macOS tasks stay open until a session on the Mac; archive only after they are done.
