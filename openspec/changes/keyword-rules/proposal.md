## Why

Three audit findings make keywords fail in ways the user cannot see. A keyword
that is the start of another keyword makes the longer one unreachable (A5-10).
Keywords expand inside Dango's own launcher search field (A5-05). On a German
keyboard on Windows, every AltGr character (`@ { } [ ] \ ~ |` and the euro
sign) clears what is held, so a keyword using one can never expand (A3-02).

## What Changes

- Saving a snippet refuses a keyword that starts with another snippet's
  keyword, or that another snippet's keyword starts with.
- Keystrokes typed into Dango's own launcher are not observed for expansion.
- On Windows, a key pressed with AltGr (right Alt) counts as text when the
  layout gives it a character, instead of clearing what is held.
- The keyword-expansion spec states that macOS still clears on an Option
  chord, as a known gap.

## Non-goals

- No change to the matcher itself (it still fires on the first keyword the
  buffer ends with).
- No check of keywords that were written into the records file by hand. Only a
  save through the form is validated.
- No fix for Option characters on macOS. They need their own change, made and
  checked on a Mac.
- No support for layouts other than English and German.

## Platforms

- A5-10: platform-neutral.
- A5-05: Windows and macOS. Each detects "typing into Dango" its own way.
- A3-02: Windows only. macOS keeps today's behaviour (an Option chord clears),
  stated in the spec as a gap, because the fix needs a session on a Mac and
  the audit decision covers Windows.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `snippets`: a keyword may not start, or be the start of, another snippet's
  keyword.
- `keyword-expansion`: AltGr characters on Windows are text; the Option gap on
  macOS is stated; typing in Dango's own launcher never expands.

## Impact

- `src/extensions/snippets/store.rs`: `validate_keyword`, a new
  `RecordError` variant and its message.
- `src/extensions/snippets/service.rs`: the worker skips keystrokes aimed at
  Dango.
- `src/platform/windows/keymonitor.rs`: `translate` handles right Alt.
- `src/platform/windows/mod.rs` and `src/platform/macos/keymonitor.rs`: how
  each platform knows the keystroke goes to Dango.
- No change to the extension manifest, the view protocol, or stored data.
