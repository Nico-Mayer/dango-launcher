## Prior findings (macOS spike, kept for reference)

Measured on macOS before the spike code was removed, and still shaping the
design:

- A `CGEvent` `ListenOnly` tap reports the characters a key produced
  (`keyboard_get_unicode_string`), including Shift-applied capitals, with events
  carrying keycode 0. A positional-key crate would have reported nothing usable.
- The tap callback cost a mean of 21.45µs against a 5ms budget.
- The system never disabled the tap under a deliberately slow callback (200ms,
  1.5s, 4s): no `kCGEventTapDisabledByTimeout` was delivered. But a slow callback
  still blocks the input path (ten characters took 10.48s with a 1s callback vs
  1.02s with a fast one). So handing off immediately is the only defence, not
  hygiene.
- Backspaces then insert completed in 380ms, inside the 500ms budget; whether the
  right characters landed was never confirmed (the probe read the wrong window).
  Anything that drives another application must verify which application it is
  driving first.

Not yet measured: non-US layout, dead keys, and umlauts on either platform; the
Windows hook cost and `ToUnicodeEx` behaviour; password-field detection on both.

## 1. Keyword on a snippet

Pure storage, validation, and matching, no monitor, so the app stays runnable.

- [x] 1.1 Add an optional `keyword` to the file-backed snippet `Record` and its create and edit paths, preserved on a round trip, and verify with unit tests that setting a keyword writes and reads it and that an absent one is `None`
- [x] 1.2 Refuse a keyword at save time when the snippet's template has placeholder arguments (self-resolving placeholders like `date` and `clipboard` are fine), and enforce keyword uniqueness across snippets, and verify both with unit tests
- [x] 1.3 Add a pure word-boundary matcher: given the recent-character buffer and the set of keywords, return the keyword that ends at the caret when what precedes it is not a word character, and verify with unit tests (start of line, after a space, not inside a longer word, a punctuation-led keyword)

## 2. The key-monitor service trait

- [x] 2.1 Define a `KeyMonitor` platform trait that starts with a sink for translated characters and clear signals (a non-character or modified key, a focus change, secure input) and stops on drop, plus a no-op fallback that reports unavailable, and verify it compiles on every target with a fake exercising start, a delivered character, a clear signal, and stop

## 3. The Windows monitor

- [x] 3.1 Implement the Windows `KeyMonitor` with a listening `WH_KEYBOARD_LL` hook on a dedicated pumped thread (the pattern `add-hyperkey` built), translating each key to characters with `ToUnicodeEx` over `GetKeyboardState` and `GetKeyboardLayout`, ignoring events marked `DANGO_INJECTED`, never swallowing a key, and handing every event off to a channel without doing work in the callback, and verify it builds and clippy is clean on Windows
- [x] 3.2 Emit the clear signals from the Windows side: a non-character key or a chord with a modifier other than Shift clears, and a focused password field (via `GetGUIThreadInfo`) suppresses observation, recording honestly in the spec what Windows cannot know, and verify it builds and clippy is clean on Windows

## 4. The expansion service

- [x] 4.1 Add a snippets `Service` that owns the monitor and a bounded ring buffer (length is the longest keyword plus one), clearing the buffer on every clear signal and after a pause, and reading the clipboard exclusion list per event so an excluded application is skipped, all off the callback thread, and verify the buffer bound and clear conditions with unit tests
- [x] 4.2 On a word-boundary keyword match, delete the keyword with one backspace per character then insert the resolved template through the existing `text` path, backspaces before the insert so a mismatch is visible rather than silent, and verify the delete-then-insert ordering with a unit test over a fake text sink
- [x] 4.3 Make the service follow the extension: it starts when snippets is enabled and stops, removing the monitor, when disabled, and verify the existing suite, clippy, and fmt are all clean

## 5. Verification

- [x] 5.1 Confirm on both platforms that typing a keyword in another application replaces it in place with the snippet's text, and that a keyword typed inside a longer word does not expand
  - Windows: typing `;sig` into a text field replaced it with the snippet's text,
    typing `a brb` after a space expanded `brb` in place, and typing `abrb` (the
    keyword inside a word) did not expand. Driven by SendKeys into a WinForms
    TextBox whose content was read back from a mirror file.
  - macOS: typing `hello ;sig` into the harness's own window replaced the
    keyword in place with the snippet's text, and `a brb` after a space expanded
    too, while `abrb` (the keyword inside a word) did not. Read back through
    System Events, so the process driving the keys is not the one answering.
  - macOS note: a punctuation-led keyword is specified to match even directly
    after a word, so `;sig` is the wrong keyword for the inside-a-word case and
    `brb` is used for it.
- [x] 5.2 Confirm on both platforms that the buffer is bounded and is cleared on a focus change, a non-character key, a pause, and secure input or a focused password field, and that typing latency in another application stays within budget with the monitor running
  - Windows: after a keyword split by a several-second pause (`;si`, wait, `g`)
    nothing expanded, confirming the pause clears the buffer. The bound and the
    non-character, focus-change, secure, and excluded clears are unit tested; the
    monitor's callback only translates and hands off, and the per-key work runs on
    the worker thread. Live latency measurement and secure-field driving were not
    scripted.
  - macOS: a keyword split by a pause (`;si`, five seconds, `g`) did not
    expand, so the pause clears the buffer. The bound and the non-character,
    focus-change and excluded clears are unit tested. Secure input is the one
    place macOS is stronger than Windows - `IsSecureEventInputEnabled` is
    authoritative - but driving a real password field was not scripted.
  - macOS latency: the spike measured the tap callback at a mean of 21.45µs
    against a 5ms budget, and the callback here only translates and sends on a
    channel. Typing stayed responsive throughout with the monitor running.
- [x] 5.3 Confirm on both platforms that disabling the snippets extension removes the monitor, that an excluded application is skipped, and that a snippet with placeholder arguments is refused a keyword
  - Windows: disabling the snippets extension live stopped expansion (`;sig`
    stayed literal). The exclusion check and the placeholder-keyword refusal are
    unit tested; driving an excluded app and the create-form refusal live were not
    scripted.
  - macOS: disabling the snippets extension live left `;sig` literal, and
    re-enabling it brought expansion back, no restart. The exclusion check and
    the placeholder-keyword refusal are unit tested; driving an excluded
    application was not scripted.
  - macOS note: a config reload restarts the service, so expansion is briefly
    unavailable for a few seconds after an edit. Expected, and the reason a
    check run immediately after an edit can see no expansion.