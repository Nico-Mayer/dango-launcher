## Context

See proposal.md - Why. What already exists and shapes this:

- The Windows hyperkey (`platform/windows/hyperkey.rs`) is a `WH_KEYBOARD_LL`
  hook on a pumped thread. On the mapped key's down it holds the configured
  modifiers via `SendInput` (marked `DANGO_INJECTED`) and swallows the key; on its
  up it releases them. It tracks a `HELD` bool so an auto-repeat down does not
  re-press. Win is pressed first and released last so a lone tap does not open the
  Start menu.
- The config `Hyperkey` type has `key` and `shift`; the emit set comes from
  `Config::hyper_modifiers()`.

## Goals / Non-Goals

**Goals:**

- A quick, solitary tap sends the tap key; a hold or a chord does not.
- A fast `hyper+X` chord is never missed, so the decision cannot wait on a timer
  before producing the modifiers.

**Non-Goals:**

- See proposal.md. No tunable timeout, no chorded tap.

## Decisions

### Emit on press, decide the tap on release

The dual-role decision cannot be made on press, because we do not yet know if the
key will be held or tapped, and delaying the modifiers until a timeout would make
a fast `hyper+X` miss. So the modifiers are produced on press exactly as today,
and the tap is decided on release from two facts recorded during the press:

- the instant of the press, to measure how long the key was held, and
- whether any other key went down while it was held.

On release, after the modifiers are let up, the tap key is sent only if the hold
was shorter than a threshold and no other key was seen. A hold longer than the
threshold, or any intervening key, sends nothing.

The threshold is a fixed ~200ms, the range Karabiner and similar tools use. It is
not configurable in this change; a constant keeps the config small and can become
a field later if it ever needs tuning.

Rejected: waiting for the threshold before emitting the modifiers. It is the
textbook dual-role approach, but it delays every chord by the threshold and drops
the first key of a fast chord, which is the opposite of what a hyperkey is for.

### The other-key flag is set in the same hook

The hook already sees every key. While the hyperkey is held, any non-injected
key-down for a key other than the trigger sets an "other key seen" flag, which is
what distinguishes a chord or incidental typing from a solitary tap. Injected
events (carrying `DANGO_INJECTED`, including the hyperkey's own modifiers) never
count. The flag is reset on each fresh press of the trigger.

### The tap key reuses the emit path

The tap key is sent with the same `SendInput` down-then-up, marked
`DANGO_INJECTED`, that the modifiers use, after the modifiers have been released
so it arrives clean. The config names it in the launcher's key grammar (for
example `escape`), resolved to a virtual key; an unresolvable name disables the
tap and is reported.

## Risks / Trade-offs

- **A tap held right at the threshold is ambiguous.** → A fixed threshold draws
  the line somewhere; ~200ms is comfortably above a deliberate tap and below a
  deliberate hold.
- **The modifiers are briefly pressed on every tap.** → They are released before
  the tap key is sent, and the Start-menu ordering already keeps a lone Win press
  from doing anything, so a tap's brief modifier press has no effect.
- **A key injected by another tool while the hyperkey is held.** → It arrives as
  a normal key-down and sets the other-key flag, so it correctly suppresses the
  tap, same as the user's own key.

## Migration Plan

Additive. An absent `tap` field is the default and preserves today's behavior, so
existing configs are unaffected.
