## 1. Find out what the editors actually expose

- [ ] 1.1 Write a throwaway probe (an ignored test or a small example) that takes the focused element through UI Automation and prints whether it has a text pattern and what its selection is, and run it against Zed in Helix mode, Zed in normal mode, Notepad, a browser text field, and Windows Terminal
- [ ] 1.2 Record the answer in the design under a "What the probe found" heading, including which of those applications need the fallback, and confirm the rest of the plan still holds before building it

## 2. The three-way answer

- [ ] 2.1 Change `DirectSelection::selected_text` to return `Selected` (text, empty, or unavailable) and update `MacSelection` to answer `Unavailable` where it answers `None` today, keeping macOS behaviour identical, with the existing text tests passing unchanged
- [ ] 2.2 Teach `TextExchange::selection` the new rule: text is the answer, empty is an empty selection with no keystroke sent, and only unavailable falls through to the clipboard, with tests for all three against a fake `DirectSelection`

## 3. Reading the selection through UI Automation

- [ ] 3.1 Add the `uiautomation` dependency and a Windows `DirectSelection` implementation that takes the focused element, reads its text pattern's first selected range, and maps a missing pattern to unavailable and an empty range to empty, and verify `cargo build` and clippy stay clean on Windows
- [ ] 3.2 Run the call on a worker thread with a 200ms deadline, answering unavailable when it does not return in time, with a test that a slow implementation does not exceed the budget
- [ ] 3.3 Wire it into `platform::text_exchange` for Windows the way `MacSelection` is wired for macOS, and verify a selection in Notepad is read with no keystroke sent and the clipboard untouched

## 4. Refusing the fallback

- [ ] 4.1 Add the `selection.excluded-applications` setting to the config, reusing the clipboard history's matcher, defaulting to empty, with a round-trip test and a test that an empty list excludes nothing
- [ ] 4.2 Refuse the clipboard fallback for a named application, reporting "Dango can't read the selection in {application}. Copy the text first, then run this command.", with a test that no keystroke is sent and one that a named application exposing its selection is still read directly
- [ ] 4.3 Document the setting in `docs/config.example.jsonc` and `docs/config.schema.json`, and verify the example still validates

## 5. Verification on both platforms

- [ ] 5.1 Verify on Windows: selecting text in Zed's Helix mode and running an AI command reads the selection, or, if Zed exposes nothing, that naming it leaves the buffer untouched and says why. Either way the block must not be commented out
- [ ] 5.2 Verify on Windows: Notepad, a browser text field, and Windows Terminal each read correctly, and that the clipboard holds what it held before in every case
- [ ] 5.3 Verify on Windows: an application with nothing selected reports an empty selection without a keystroke, and one with no text pattern still works through the fallback
- [ ] 5.4 Verify on Windows: a 1,000 character selection is returned within the 300ms the spec gives it, measured rather than estimated
- [ ] 5.5 Verify on macOS: reading a selection, pasting, snippets, and keyword expansion all behave exactly as before, since the trait they share changed shape
