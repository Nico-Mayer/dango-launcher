## 1. Find out what the editors actually expose

- [x] 1.1 Write a throwaway probe (an ignored test or a small example) that takes the focused element through UI Automation and prints whether it has a text pattern and what its selection is, and run it against Zed in Helix mode, Zed in normal mode, Notepad, a browser text field, and Windows Terminal
- [x] 1.2 Record the answer in the design under a "What the probe found" heading, including which of those applications need the fallback, and confirm the rest of the plan still holds before building it

## 2. The three-way answer

- [x] 2.1 Change `DirectSelection::selected_text` to return `Selected` (text, empty, or unavailable) and update `MacSelection` to answer `Unavailable` where it answers `None` today, keeping macOS behaviour identical, with the existing text tests passing unchanged
- [x] 2.2 Teach `TextExchange::selection` the new rule: text is the answer, empty is an empty selection with no keystroke sent, and only unavailable falls through to the clipboard, with tests for all three against a fake `DirectSelection`

## 3. Reading the selection through UI Automation

- [x] 3.1 Add the `uiautomation` dependency and a Windows `DirectSelection` implementation that takes the focused element, reads its text pattern's first selected range, and maps a missing pattern to unavailable and an empty range to empty, and verify `cargo build` and clippy stay clean on Windows
- [x] 3.2 Run the call on a worker thread with a 200ms deadline, answering unavailable when it does not return in time, with a test that a slow implementation does not exceed the budget
- [x] 3.3 Wire it into `platform::text_exchange` for Windows the way `MacSelection` is wired for macOS, and verify a selection in Notepad is read with no keystroke sent and the clipboard untouched

## 4. Refusing the fallback

- [x] 4.1 Add the `selection.excluded-applications` setting to the config, reusing the clipboard history's matcher, defaulting to empty, with a round-trip test and a test that an empty list excludes nothing
- [x] 4.2 Refuse the clipboard fallback for a named application, reporting "Dango can't read the selection in {application}. Copy the text first, then run this command.", with a test that no keystroke is sent and one that a named application exposing its selection is still read directly
- [x] 4.3 Document the setting in `docs/config.example.jsonc` and `docs/config.schema.json`, and verify the example still validates

## 5. Verification on both platforms

- [x] 5.1 Verify on Windows: selecting text in Zed's Helix mode and running an AI command reads the selection, or, if Zed exposes nothing, that naming it leaves the buffer untouched and says why. Either way the block must not be commented out
  - Zed exposes nothing, so it is the naming case. With Zed named, running
    Improve Writing left the buffer byte-identical (checked against git, and by
    eye at the end of the file) and the banner read "Dango can't read the
    selection in Zed. Copy the text first, then run this command."
  - The first attempt at this failed and toggled Zed's comments again, which is
    what found the foreground bug now recorded in the design: the refusal was
    being decided while the launcher held the foreground, so it never matched.
- [ ] 5.2 Verify on Windows: Notepad, a browser text field, and Windows Terminal each read correctly, and that the clipboard holds what it held before in every case
  - Notepad: done, end to end. A selection was read and a command routed through
    it answered, with the clipboard holding what it held before the read.
  - Browser text field: done through the probe. The address bar answers with its
    selection in about 2ms.
  - Windows Terminal: not exercised. It is not installed on this machine, so
    `wt.exe` could not be started.
- [x] 5.3 Verify on Windows: an application with nothing selected reports an empty selection without a keystroke, and one with no text pattern still works through the fallback
  - Nothing selected: a collapsed cursor in Notepad answers with an empty range,
    which is an empty selection rather than the whole document. Worth stating
    plainly, because the other reading would have transformed the entire file.
  - No text pattern: page content in a browser has none, and a command run
    against a page selection read it through the clipboard fallback as before.
- [x] 5.4 Verify on Windows: a 1,000 character selection is returned within the 300ms the spec gives it, measured rather than estimated
  - Measured over four samples of a ~1,000 character selection in Notepad:
    10.1ms, 1.66ms, 1.67ms, 1.65ms. The first call pays for COM initialisation.
    Well inside both the 200ms deadline and the 300ms budget.
- [ ] 5.5 Verify on macOS: reading a selection, pasting, snippets, and keyword expansion all behave exactly as before, since the trait they share changed shape
