## Why

Milestone: M5 (keys).

`add-hyperkey` turns CapsLock into the hyper modifier, but a tap of it does
nothing, which wastes the most reachable key on the keyboard. A dual-role key,
Escape on a tap and hyper on a hold, is how Karabiner and Hyperkey use CapsLock,
and it puts Escape back within reach on keyboards that bury it. The earlier
change listed tap-to-act as a non-goal "reserved for a later change"; this is it.

## What Changes

- The hyperkey gains an optional tap action: a quick press-and-release, with no
  other key pressed in between, sends a configured key (Escape by intent). A hold,
  or a press combined with another key, still acts as the hyper modifier as
  before.
- Add an optional `tap` field to the hyperkey config naming the key to send on a
  tap. Absent means a tap does nothing, today's behavior, so existing configs are
  unchanged.
- The modifiers are still emitted on press, so a fast `hyper+X` chord never
  misses. Whether a press was a tap is decided on release, from how briefly it was
  held and whether any other key was pressed while it was down.

## Capabilities

### Modified Capabilities

- `hyperkey`: a tap of the hyperkey MAY send a configured key; the previous "a
  lone tap does nothing" becomes "a lone tap sends the tap key, or nothing when
  no tap key is configured."

## Impact

- Touches the hyperkey config type and schema, the example docs, and the Windows
  hyperkey hook (`platform/windows/hyperkey.rs`), which gains press-timing and an
  other-key-seen flag and sends the tap key on a qualifying release. macOS gains
  the same field and behavior when its hyperkey lands.
- No new dependency. The tap key is synthesized through the same `SendInput`
  path, marked `DANGO_INJECTED`, that the modifiers already use.

## Non-goals

- No change to the hold behavior or the configured modifier set.
- No per-application tap behavior, and no chorded tap (a tap is one key).
- No configurable tap timeout in this change; a single sensible threshold is
  used, and making it tunable can follow if it is ever needed.

## Platforms

- Windows and macOS both by design. Windows is implemented and verified here;
  macOS follows with the rest of its hyperkey, its verification deferred, matching
  the project's split.
