## Context

See proposal.md for why. One parser, `parse_with_hyper` in
`src/config/hotkey.rs`, reads every hotkey: launcher, command, application.
Its `MODIFIER_TOKENS` and `KEY_TOKENS` tables also build the schema pattern and
the settings reference in `src/config/schema.rs`. A parse error already reaches
the user: `hotkeys::launcher` falls back to Alt+Space / Option+Space and
`register_hotkeys` logs it and pushes the "Invalid launcher.hotkey, using
{shortcut}, see dango.log" notice; `Table::claim` turns a command or
application parse error into a conflict line for the tray status and the log.
So the fix is a parser rule plus a matching pattern; the reporting path stays.

## Goals / Non-Goals

**Goals:** one rule, enforced by the parser, mirrored by the schema, and kept
in step by tests.

**Non-Goals:** a lone modifier key as a hotkey; a keyboard hook.

## Decisions

### Function keys F1 to F20, not F1 to F24

Checked against the vendored `global-hotkey` 0.8.0:

- Windows (`platform_impl/windows/mod.rs`, `key_to_vk`) maps F1 to F24 to
  `VK_F1`..`VK_F24` and calls `RegisterHotKey` with `MOD_NOREPEAT` plus
  whatever modifiers are set, so zero modifiers is passed through.
- macOS (`platform_impl/macos/mod.rs`, `key_to_scancode`) maps F1 to F20 only;
  F21 to F24 fall to `_ => None`, and `register` then fails. Carbon has no
  virtual key code past `kVK_F20`. `RegisterEventHotKey` takes a zero
  modifier mask.

So F1 to F20 register on both platforms, alone or with modifiers.

Alternatives rejected:
- F1 to F24 with F21 to F24 Windows-only. The grammar is shared and a portable
  chord should mean the same on both machines; a per-platform key list also
  makes the schema pattern platform-dependent. F21 to F24 keys are rare.
- Keep F1 to F12. F13 to F20 are the keys most worth binding alone: they exist
  on Mac extended keyboards and as remap targets, and nothing else uses them.

### Lone modifier keys stay refused

A lone Windows key or Command key cannot go through `RegisterHotKey` or
`RegisterEventHotKey`; it needs a low-level keyboard hook or a `CGEventTap`
(Input Monitoring on macOS). The author dropped it. The parser already returns
`NoKey` for a modifier in the key position, so no change.

### The check lives in the parser

`parse_with_hyper` returns a new `HotkeyError::NoModifier` when the modifier
set is empty and the key is not F1 to F20. Doing it in the parser means the
launcher, commands, and applications all get it, and the existing reporting
paths carry it unchanged. Rejected: checking in `hotkeys.rs` before
registration, which would leave `parse` and the schema disagreeing again.

A `hyper` token whose configured set is empty is not possible today (the
hyperkey always emits at least one modifier), so the check runs on the
expanded set without a special case.

### Schema pattern

The pattern becomes two alternatives: the existing
`(?:(?:mods)\+)+(?:keys|[a-z0-9])` or a lone `f(?:[1-9]|1[0-9]|20)`. The lone
alternative is built from the same `KEY_TOKENS` table, filtered to function
keys, so adding a key cannot drift. `every_token_matches_the_hotkey_pattern`
gains a check that every parser-accepted lone key matches and a sample of
refused bare keys does not.

### Copy

No new tray text. The new log reason, from the `HotkeyError` display, follows
the style of the existing variants:

- `"the hotkey has no modifier; only F1 to F20 can stand alone"`

It reaches the user inside the existing lines, for example
`"invalid launcher hotkey, using the default: the hotkey has no modifier; only F1 to F20 can stand alone"`
and `"<target>: invalid hotkey, the hotkey has no modifier; only F1 to F20 can stand alone"`.

The settings reference line becomes:

- "A chord is one or more modifiers joined with `+`, then one key, all
  lowercase. A function key from `f1` to `f20` can also stand alone."

No contract change: the extension manifest and the view protocol are not
touched.

## Risks / Trade-offs

- [On macOS the hyperkey remaps CapsLock to F18 through `hidutil`, and its event
  tap consumes F18, so an `f18` hotkey likely never fires while the hyperkey is
  on, with no notice] → Confirm in the macOS hand check (task 3.2), then either
  refuse `f18` on macOS while the hyperkey is on, or document the clash.
- [Windows reserves F12 for the debugger; `RegisterHotKey` may refuse a lone
  F12] → the existing refusal path surfaces it, as for any taken chord.
- [On Mac laptops F1 to F12 send media keys unless Fn is held or the system
  setting is changed, so a lone F5 may never reach Dango] → no detection;
  F13 to F20 avoid it.
- [A config that used a bare key stops working] → alpha, no migration; the
  notice says what happened.
