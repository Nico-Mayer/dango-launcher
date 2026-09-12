## 0. Status: deferred

Not being built. The author dropped it to finish M3 first, and the spike had
already shown it is a bigger piece of work than it looks. The planning artifacts
and the findings below are kept so that picking it up again starts from what was
measured rather than from scratch.

The spike code itself was removed, along with the `CFMachPort`, `CFRunLoop`,
`CGEvent` and `CGEventTypes` features it needed. Everything below was measured
on macOS before it went.

## 1. Spike: what the platforms actually deliver

The last change's spike overturned two design decisions and its probe measured
nothing on the first attempt. These are the questions that would be expensive to
answer after the monitor is built.

- [x] 1.1 On macOS, install a `CGEvent` tap and confirm `keyboard_get_unicode_string` returns the characters a key produced, checked against a non-US layout, a dead key, and a Shift chord
  - **Confirmed for the mechanism.** A `ListenOnly` tap reports `a`, `Z`, `7`, `;`, `x` for those keystrokes, including the Shift-applied capital.
  - The evidence for the dependency decision is sharper than expected: every event carried **keycode 0**, because the events were synthesised with a unicode string and no meaningful key position, and the characters still came back correctly. A crate reporting positional keys would have reported nothing usable for any of them.
  - Not done: the non-US layout, dead key, and umlaut checks. Those need typing by hand on a German layout.
- [x] 1.2 On macOS, measure what the tap callback costs per keystroke, and confirm typing in another application stays responsive with it installed
  - 60 callbacks, mean **21.45µs**, against the 5ms per keystroke the spec allows. Roughly 230 times of headroom, and that includes reading the characters out of the event.
- [x] 1.3 On macOS, confirm the tap is disabled by the system under a deliberately slow callback, and that re-enabling on `kCGEventTapDisabledByTimeout` recovers it
  - **The design had this backwards, and the correction makes it worse rather than better.** The system never disabled the tap: not at 200ms per callback, not at 1.5s, not at 4s. No `kCGEventTapDisabledByTimeout` was ever delivered.
  - But a slow callback still blocks the input path, measured directly: ten characters took **1.02s** with a fast callback and **10.48s** with a 1000ms one.
  - So there is no rescue and no signal. A slow callback does not get disabled, it just makes the whole machine slow, silently. The design called the tap being disabled "the most likely field failure"; the real failure is that it is never disabled. Handing off immediately stops being hygiene and becomes the only defence, and design.md should say so before any of this is built.
- [ ] 1.4 On macOS, confirm `IsSecureEventInputEnabled` reports true while typing into a password field, in a native application and in a browser
- [ ] 1.5 On macOS, confirm sending backspaces then inserting replaces a typed word cleanly, including in an application that autocorrects
  - Partly measured: the backspaces plus the insert complete in **380ms**, inside the 500ms the spec allows. Whether the right characters ended up in the document was never confirmed, because the probe read back from the wrong window.
  - **A probe bug worth remembering.** `open -e` was assumed to have focused the scratch document, and it had not. The select-all and copy that followed therefore read the frontmost window, which was a browser, and printed a page of the author's own content into the terminal. The M3 harness avoids this by checking the frontmost application before it does anything; this probe did not, and should have. Anything that drives another application has to verify which application it is driving.
- [ ] 1.6 On Windows, confirm `WH_KEYBOARD_LL` with `ToUnicodeEx` returns characters for a non-US layout, and measure what the hook costs on the input path
- [ ] 1.7 On Windows, establish what can be known about a password field, and record honestly what cannot
- [ ] 1.8 If any of the above does not work, revise design.md before building on it

## 2. Keywords on a snippet

Pure storage and validation, no monitor, so the app stays runnable.

- [ ] 2.1 Add the migration for a nullable `keyword` column with a unique index over live rows, and verify it applies to an existing database without touching the other tables
- [ ] 2.2 Store, read back, and clear a keyword, with tests
- [ ] 2.3 Refuse a keyword another snippet already has, with a test naming the conflict
- [ ] 2.4 Refuse a keyword on a snippet whose template needs arguments, with a test and a message explaining why
- [ ] 2.5 Accept a keyword on a snippet using only reserved placeholders, with a test over one using `date`
- [ ] 2.6 Add the keyword field to the create and edit forms, verified by creating a snippet with one
- [ ] 2.7 Show a snippet's keyword where the snippet appears, verified by eye

## 3. The buffer and the matcher

Everything about deciding that a keyword was typed, with no platform underneath,
so it can be tested exhaustively.

- [ ] 3.1 Add the ring buffer bounded by the longest keyword, with a test that a long passage never grows it
- [ ] 3.2 Match a keyword only at a word boundary, with tests for start of input, after a space, after punctuation, and inside a longer word
- [ ] 3.3 Match exactly, with a test that a differing character or case does not match
- [ ] 3.4 Clear on a non-character key, with a test per class: Enter, Tab, Escape, an arrow, and a modifier chord
- [ ] 3.5 Clear on a focus change, with a test
- [ ] 3.6 Clear after a pause, with a test driven by an injected clock
- [ ] 3.7 Clear when secure input becomes active, with a test
- [ ] 3.8 Reread the keywords when snippets change, so a new keyword works without a restart, with a test
- [ ] 3.9 Verify matching a keystroke against 500 keywords stays under 1ms, with a test carrying the budget

## 4. Expansion

- [ ] 4.1 Define the `KeyMonitor` trait covering starting, stopping, the character stream, and whether secure input is active, with a fake for testing
- [ ] 4.2 Send one backspace per keyword character before inserting, with a test over a multi-byte keyword that counts characters rather than bytes
- [ ] 4.3 Insert through the existing `TextExchange`, with a test that the clipboard is declared and restored exactly as a launcher insertion is
- [ ] 4.4 Skip expansion entirely in an excluded application, reading the list per event, with a test
- [ ] 4.5 Report a failed expansion without leaving the keyword half-deleted, with a test
- [ ] 4.6 Run the matcher off the callback thread, with a test that the callback path does no database work

## 5. Platform: macOS

- [ ] 5.1 Install the event tap on its own thread with its own run loop, and stop it cleanly, verified by starting and stopping repeatedly without leaking a tap
- [ ] 5.2 Translate events to characters through `keyboard_get_unicode_string`, verified by hand against the layouts from 1.1
- [ ] 5.3 Re-enable the tap on `kCGEventTapDisabledByTimeout` and `kCGEventTapDisabledByUserInput`, verified by provoking it as in 1.3
- [ ] 5.4 Report secure input through `IsSecureEventInputEnabled`, verified against a password field
- [ ] 5.5 Verify the tap observes without swallowing: every keystroke still reaches the application, including ones that complete a keyword
- [ ] 5.6 Verify expansion does nothing when Accessibility has not been granted, and says why

## 6. Platform: Windows

Cannot be compiled on macOS. Lands as code plus confirmations the author closes
on the Windows machine, the arrangement the last two changes used.

- [ ] 6.1 Install `WH_KEYBOARD_LL` on a thread with a message pump, and remove it cleanly on stop
- [ ] 6.2 Translate to characters with `ToUnicodeEx`, `GetKeyboardState` and the foreground layout, verified against the layouts from 1.6
- [ ] 6.3 Keep the hook callback free of anything that can block, verified by typing continuously in another application
- [ ] 6.4 Apply whatever password-field signal 1.7 established, and document the limit where there is none
- [ ] 6.5 Verify the hook observes without swallowing
- [ ] 6.6 Check symbol names and module paths against the vendored crate source before pushing, and verify CI's `cargo clippy -- -D warnings` passes on `windows-latest`

## 7. The service

- [ ] 7.1 Declare the monitor as a service on the snippets extension, started and stopped with the extension, with a test
- [ ] 7.2 Verify disabling the extension removes the monitor rather than leaving it installed and idle
- [ ] 7.3 Verify enabling it again starts expanding without a restart

## 8. Verification

- [ ] 8.1 Walk every scenario in the two spec files on macOS
- [ ] 8.2 Walk every scenario in the two spec files on Windows
- [ ] 8.3 Confirm on both platforms that a keyword expands in at least four applications: a native one, a browser, an Electron one, and a terminal
- [ ] 8.4 Confirm on both platforms that typing stays responsive with the monitor running, measured rather than judged
- [ ] 8.5 Confirm on both platforms that nothing typed reaches the database, a file, or a log, by inspecting all three after typing a distinctive string
- [ ] 8.6 Confirm on macOS that typing a password into a password field is not observed, and record what Windows can and cannot promise
- [ ] 8.7 Confirm on both platforms that an excluded application never expands
- [ ] 8.8 Confirm on both platforms that the clipboard and the clipboard history are untouched by an expansion
- [ ] 8.9 Confirm on both platforms that activation still meets the 80ms budget with the monitor running
- [ ] 8.10 Confirm on macOS that provoking the system into disabling the tap is recovered from without a restart
- [ ] 8.11 Confirm expansion does not fire inside the launcher's own window, which design.md leaves open
