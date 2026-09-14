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
- [x] 5.2 Verify on Windows: Notepad, a browser text field, and Windows Terminal each read correctly, and that the clipboard holds what it held before in every case
  - Notepad: done, end to end. A selection was read and a command routed through
    it answered, with the clipboard holding what it held before the read.
  - Browser text field: done through the probe. The address bar answers with its
    selection in about 2ms, and browser page content answers through
    `Chrome_RenderWidgetHostHWND` rather than not at all, which corrects the
    earlier note that page content exposes nothing: the element sampled then was
    a video player, not text.
  - Windows Terminal: done through the probe, with two caveats worth keeping.
    Its focused element is `TermControl/Text` and it does expose a text pattern,
    answering with the buffer selection in 1.7ms to 11ms.
    - A selection made at the prompt with Shift+Home is invisible to it. That is
      a PSReadLine input selection, which the shell renders itself; UI Automation
      reports the terminal buffer, so only a buffer selection (mark mode, or the
      mouse) is read. Not a defect, but it means "selected" in a terminal means
      something narrower than elsewhere.
    - The end-to-end through the launcher was not driven. Mark mode holds the
      keyboard, so the launcher chord never reaches the system while a buffer
      selection is being made, and a mouse-driven attempt was abandoned after it
      clicked on the wrong window. The read itself is the part this change
      touches, and that is measured above.
  - The window is titled by its profile ("PowerShell"), not "Terminal", which is
    why an earlier search for it found nothing and reported it as not installed.
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
- [x] 5.5 Verify on macOS: reading a selection, pasting, snippets, and keyword expansion all behave exactly as before, since the trait they share changed shape
  - The baseline did not build on macOS, which is the finding this task exists
    to catch. `MacSelection::selected_text` still returned `Option<String>`
    against a trait that now returns `Selected`, and the macOS walkthrough had
    never been given the `refuses` argument the Windows one got. Both are the
    macOS half of work ticked off in 2.1 and 4.2: only the Windows side is
    compiled on Windows, so neither error could show up there. Fixed as 2.1
    specifies, mapping `None` to `Unavailable`, not worked around.
  - Verified with the walkthrough harness against TextEdit, which drives the
    real `TextExchange`: 20 checks, all passing, with the AI and exclusion legs
    enabled. Insertion, the clipboard restore, an image on the clipboard, the
    declared writes, and the caret all behave as they did.
  - The read takes the direct route, measured rather than assumed. The clipboard
    fallback declares its sentinel write to the history and the accessibility
    route declares nothing, so a read that declares nothing never reached the
    clipboard. The harness now asserts that, which is what tells this apart from
    a fallback that happens to return the same text.
  - The launcher stays on screen. `MacSelection::needs_foreground` is false, so
    `selection()` never dismisses, and two tests now pin it: one that a direct
    route which does not need the foreground leaves the launcher up, the mirror
    of the existing Windows-shaped one, and one on `MacSelection` itself.
    Nothing had pinned either before.
  - A template using `{{ selection }}` renders and arrives, and a typed keyword
    still expands in place with the clipboard given back. Keyword expansion goes
    through `expand`, which nothing in this harness reached before.
  - An AI command reading the selection answers, through Ollama on `gemma3:4b`,
    driving the whole composition: read the selection, render the shipped
    Improve Writing prompt around it, ask the model.
  - The exclusion list was verifiable after all. Zed exposes nothing through
    `AXSelectedText` on macOS either, with a full selection made and the
    application element asked directly, so it is the same case it was on
    Windows. Named in `selection.excluded-applications` it produced exactly
    "Dango can't read the selection in Zed. Copy the text first, then run this
    command." and declared no clipboard write, so no keystroke was sent.
  - One asymmetry worth stating, deliberate rather than a regression. On macOS
    an empty selection still falls through to the clipboard: `AXSelectedText`
    answers with an empty string, which is filtered to `None` and so becomes
    `Unavailable`. Windows tells those two apart and macOS does not, which is
    exactly what 2.1 asked for in keeping macOS identical, but it does mean the
    "answered, but empty" rule the spec states is only live on Windows.
  - The harness had its own bug, found while adding the Zed check and fixed:
    `focus` waited on `NSWorkspace` with a sleep, and the frontmost application
    only changes while a run loop is pumping, so once Zed took the front it
    could never observe TextEdit coming back. It now pumps, which is the trap
    the selection spike had already recorded.
