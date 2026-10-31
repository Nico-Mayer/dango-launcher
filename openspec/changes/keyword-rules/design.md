## Context

See proposal.md for the three findings. The code involved today:

- `Records::validate_keyword` (`src/extensions/snippets/store.rs`) refuses only
  an exact duplicate. The matcher fires as soon as the buffer ends with any
  keyword, so `;s` always wins over `;sig`.
- The keyword worker (`run` in `src/extensions/snippets/service.rs`) already
  asks `platform::foreground_app()` for every character, to clear on an app
  switch and to honour the exclusion list. It never asks whether the app is
  Dango.
- On Windows, `translate` (`src/platform/windows/keymonitor.rs`) returns
  `Clear` when `VK_CONTROL` or `VK_MENU` is down or the message is
  `WM_SYSKEYDOWN`, before it calls `ToUnicodeEx`. Windows reports AltGr as
  left Ctrl plus right Alt, so every AltGr character clears.
- On macOS, `translate` (`src/platform/macos/keymonitor.rs`) returns `Clear`
  for any event with Control, Option, or Command in its flags. On a German Mac
  layout `@` is Option+L, so macOS has the same gap for Option characters. The
  launcher is a non-activating `NSPanel`, so while the user types in it the
  frontmost app is still the previous app, and `foreground_app()` never names
  Dango.

## Goals / Non-Goals

**Goals:**

- Each finding fixed at the point that decides it, with a unit test.

**Non-Goals:**

- Changing the matcher, or validating keywords that were edited into the
  records file by hand. Such a file can still hold overlapping keywords; the
  shorter one wins, as today.
- Option characters on macOS (see proposal.md).

## Decisions

### 1. Refuse overlapping keywords at save

`validate_keyword` scans the other records once and refuses when the new
keyword starts with another record's keyword or another record's keyword
starts with it. The exact duplicate stays its own case (`KeywordTaken`), since
a duplicate is also a prefix and its message is more precise, so it is checked
first. The overlap gets a new variant, `RecordError::KeywordOverlaps`, carrying
the other keyword for tests. The comparison is case-sensitive, because the
matcher is (keyword-expansion spec, "Only the exact keyword matches").

Rejected:

- Make the matcher prefer the longest keyword. It would have to wait for the
  next keystroke or a pause before expanding `;s`, which breaks "Expansion is
  fast enough to keep typing through".
- Warn but still save. The longer keyword would still never expand, and the
  form has no warning state.

### 2. Skip keystrokes aimed at Dango

Windows: the worker asks a new `platform::foreground_is_dango()` before it
looks at the exclusion list. It compares the process id of
`GetForegroundWindow` with `std::process::id()`, which is cheaper than the
name lookup and cannot be fooled by another app named Dango. When it is true
the worker clears the buffer and skips the character, so
`note_insertion_target` is never called with the launcher.

macOS: the frontmost-app check cannot see the non-activating panel, so the
check sits in the tap instead. `translate` reads the event's
`EventTargetUnixProcessID` field and returns `Clear` when it equals Dango's own
process id. The worker then holds nothing. `foreground_is_dango()` on macOS
returns false: the panel never makes Dango frontmost.

Rejected:

- Only the own-process foreground check on both platforms. It never fires on
  macOS, for the panel reason above.
- A "launcher is shown" flag set by `Core`. It couples the snippets service to
  the app core, and "shown" is not the same as "has the keyboard".
- Filtering in the Windows hook. The hook stays as small as possible on the
  input path; the worker already does the per-character checks.

### 3. Let ToUnicodeEx decide when right Alt is down (Windows)

The chord test moves into a pure function that takes which modifiers are down
and the message, and answers either `Clear` or "translate, with or without
Ctrl+Alt". When `VK_RMENU` is down and no Win key is down, it answers
"translate with Ctrl+Alt", ignoring `VK_CONTROL`, `VK_MENU`, and
`WM_SYSKEYDOWN`. `translate` then sets `VK_CONTROL`, `VK_LCONTROL`,
`VK_MENU`, and `VK_RMENU` to `0x80` in the state array before `ToUnicodeEx`.
If the layout gives no character (US English, where right Alt is plain Alt,
or a key with no AltGr meaning) the existing rule clears. The pure function
gets the test the audit asks for.

Rejected:

- Pass Ctrl+Alt to `ToUnicodeEx` for any Ctrl+Alt chord. Left Ctrl+Alt
  shortcuts would turn into characters on a German layout.
- Use the low-level hook's `LLKHF_ALTDOWN` flag. It does not tell left from
  right Alt.

### Interface text

One new string, the message of `RecordError::KeywordOverlaps`, shown as the
form's failure toast. Written with the ux-writing skill and checked against
`openspec/specs/interface-copy` (sentence case, 13 words, says what to do, no
blame, uses "keyword"):

> One keyword can't be the start of another, so choose a different keyword

It goes into the `every_fixed_message_is_a_sentence` test list.

No change to the extension manifest or the view protocol. No general job is
hand-rolled, so no crate was evaluated.

## Risks / Trade-offs

- [macOS: at a session-level listen-only tap, `EventTargetUnixProcessID` may
  still be 0 because the event is not yet routed] -> The macOS hand check
  proves it on a Mac. If the field is empty, fall back to a flag the macOS
  window code sets when the panel becomes and stops being the key window, and
  update this design before applying.
- [Windows: Dango may also have a hidden window that is foreground briefly
  during a paste] -> Skipping keystrokes then is harmless; Dango sends those
  keys itself and the monitor already ignores injected ones.
- [Overlapping keywords in a hand-edited file stay possible] -> Accepted; the
  author edits through the form.
