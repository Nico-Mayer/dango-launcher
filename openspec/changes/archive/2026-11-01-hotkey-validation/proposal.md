## Why

The hotkey parser accepts a chord with no modifier, so `"hotkey": "space"` or
`"hotkey": "a"` registers a global hotkey that swallows that key in every
application until the config is fixed. The schema pattern already requires at
least one modifier, so the editor flags the value while the app accepts it: the
parser and the schema disagree (audit finding A8-02).

## What Changes

- A hotkey with no modifier is refused, unless its key is a function key F1 to
  F20. A lone F-key is a valid hotkey on both platforms.
- A refused hotkey is reported the way invalid hotkeys already are: a log line
  and a tray notice. An invalid launcher hotkey falls back to the platform
  default; an invalid command or application hotkey is not registered.
- The grammar gains F13 to F20 as keys, with or without modifiers.
- The schema pattern accepts exactly what the parser accepts: one or more
  modifiers plus a key, or a lone F1 to F20. The generated `docs/config.md`
  says so.
- **BREAKING** for a config that relied on a bare non-function key. Alpha, no
  migration.

## Non-goals

- A lone Windows key or Command key as a hotkey. It needs a keyboard hook, not
  `RegisterHotKey` or `RegisterEventHotKey`, and the author dropped it.
- F21 to F24. macOS has no key code for them (see design).
- Other new keys (numpad, Insert, Print Screen, media keys).
- Detecting that a lone F-key is taken by the system or by laptop media-key
  settings. Existing refusal reporting applies where the OS reports one.
- Changing the hyperkey or the in-launcher bindings.

## Platforms

Windows and macOS. The grammar and the schema are shared code; registration of
a lone F-key goes through the existing `global-hotkey` path on both.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `global-activation`: adds the rule that a hotkey needs a modifier unless its
  key is F1 to F20, and how a refused launcher hotkey is reported.
- `command-hotkeys`: a command or application hotkey with a bare non-function
  key is refused and surfaced, not registered.
- `config-schema`: the parser-and-schema agreement covers the modifier rule and
  the lone F-key case.

## Impact

- `src/config/hotkey.rs`: new `HotkeyError` variant, modifier check, F13 to
  F20 tokens, tests.
- `src/config/schema.rs`: pattern and reference text, tests.
- `docs/config.md`: regenerated.
- `src/hotkeys.rs`, `src/app/startup.rs`, `src/app/mod.rs`: no change in
  shape; they already log and surface a parse error.
