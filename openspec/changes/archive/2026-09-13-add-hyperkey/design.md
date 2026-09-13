## Context

See proposal.md - Why. What already exists and shapes this:

- Chord parsing (`config::hotkey`) already expands `hyper` to
  `CONTROL | ALT | SHIFT | SUPER`, and the launcher and command hotkeys register
  `hyper+X` chords through `tauri-plugin-global-shortcut` today. Nothing consumes
  a physical hyper key yet, so those chords are unreachable.
- The Windows text path stamps every event it injects with
  `DANGO_INJECTED = 0x44_41_4E_47` in `dwExtraInfo`, with the comment that it is
  "so M5's key monitor can tell its own output from the user's". This change is
  that consumer.
- The config is parsed into a typed `Config` that preserves unknown keys on a
  round-trip; `hyperkey` is currently only an unknown, preserved key. The config
  watcher and `apply_config_reload` in `lib.rs` already re-apply the launcher and
  command hotkeys on an edit and surface problems to `dango.log` and the tray.
- There is no platform key-interception layer yet. Platform capabilities live
  behind `cfg` traits (`SystemControl`, `WindowManager`, `Keys`/`Handoff`).

## Goals / Non-Goals

**Goals:**

- One physical key emits the hyper modifier system-wide, applied at startup and
  live, configured only from the file.
- The hyperkey produces modifiers and nothing else, so it composes with the
  existing registration without touching it.
- No stuck modifiers: whatever is synthesized down is always released.

**Non-Goals:**

- See proposal.md - Non-goals. At design level additionally: no new chord
  registration, no change to the invocation path, and no dependency on the
  global-shortcut plugin's internals.

## Decisions

### A platform `Hyperkey` service, owned like the other platform traits

A `Hyperkey` trait is started from the config and returns a running handle that
is dropped to stop. It sits behind `cfg`: a Windows implementation, a macOS
implementation, and a no-op fallback that reports "unavailable" the way key
injection already does. `lib.rs` owns the handle, starts it after the hotkeys are
registered, and replaces it on a reload.

Rejected: driving the remap from inside the global-shortcut handler. The handler
only sees registered chords after the fact; the remap has to act on raw key
events before any chord is formed.

### Windows: a low-level keyboard hook on a dedicated pumped thread

The Windows implementation installs `WH_KEYBOARD_LL` (a global hook, not tied to
a window) on its own thread running a `GetMessage` pump, because a low-level hook
delivers callbacks on the thread that installed it and needs a live message loop.

In the callback, for the mapped key:

- On key-down, if not already held: record held, synthesize key-down for the
  four left-hand modifiers (`VK_LCONTROL`, `VK_LMENU`, `VK_LSHIFT`, `VK_LWIN`)
  with `SendInput`, each stamped `DANGO_INJECTED`, then return non-zero to
  swallow the mapped key so its own function (the CapsLock toggle) never fires.
- On an auto-repeat key-down while already held: swallow it, synthesize nothing.
- On key-up: clear held, synthesize key-up for the four modifiers, swallow the
  mapped key.

Every other event, and any event whose `dwExtraInfo` is `DANGO_INJECTED`, passes
through untouched (`CallNextHookEx`). Passing our own injected modifiers through
is what stops a feedback loop, and reuses the existing marker.

The synthesized modifiers are real held key state, so when the user then presses
the trigger key, `RegisterHotKey` sees `hyper+trigger` and fires the bound
command through the path that already exists. The hook does no matching itself.

The callback stays minimal (a few `SendInput` calls) to stay under the
`LowLevelHooksTimeout` the OS enforces before it silently drops a slow hook.

Rejected: `hidutil`-style scancode remap (no Windows equivalent that yields a
modifier), and depending on PowerToys Keyboard Manager (an external tool, not in
the dotfiles, Windows-only).

### macOS: a CGEventTap rewriting the mapped key into flag changes

The macOS implementation adds a session-level `CGEventTap` that watches for the
mapped key and, while it is held, sets the four modifier flags
(`maskControl | maskAlternate | maskShift | maskCommand`) on subsequent events,
suppressing the key's own effect. This needs the Accessibility permission, like
the existing selection path.

Setting the flags on the events that pass through, rather than synthesizing
modifier key-downs the way Windows does, is what makes the macOS side free of
the stuck-modifier risk: no modifier is ever pressed, so none can be left down.
The flags exist only on events the tap has already seen, which is exactly the
window in which the hyperkey is held.

### macOS: CapsLock is remapped to F18 first, because the tap cannot own it

A spike measured what a session tap actually receives (`examples/keytap_spike.rs`):
CapsLock arrives as a `FlagsChanged` event, keycode 57, carrying
`maskAlphaShift`. The problem is what has already happened by then. The
lock state and its LED are owned by the HID system, which sits below every tap
location, so swallowing the event at a session tap does not stop the toggle; and
the same layer debounces the key, which would make a quick tap unreliable for
`add-hyperkey-tap`.

So the mapped key is first remapped at the HID level, below the toggle, with
`hidutil property --set` on `UserKeyMapping`: CapsLock (`0x700000039`) to F18
(`0x70000006D`), a key no Apple keyboard has. The tap then sees an ordinary
`KeyDown`/`KeyUp` for keycode 79, with no lock state, no LED, and no debounce,
and owns it outright. This is the same two-layer arrangement every working macOS
hyperkey uses, and it is the reason a tap alone is not enough.

The remap is applied when the hyperkey starts and cleared when the handle drops,
so disabling the hyperkey or quitting Dango gives CapsLock back. It is machine-wide
while it is set and it does not survive a reboot, which is the trade named in the
risks.

Rejected: swallowing the `FlagsChanged` at the tap with no remap. It is what this
design first assumed, and the measurement says the toggle happens anyway.

Rejected: a Karabiner-style virtual HID driver. It owns the problem properly and
it is a kernel extension and a signed installer, which is far past what a hobby
project should install on a machine.

### Config shape: an optional typed `hyperkey` block, off by default

```jsonc
"hyperkey": { "key": "capslock", "shift": true }   // present = on; absent = off
```

`key` names a physical key from a small allowlist (starting with `capslock`;
others can be added without a schema break). Presence enables it; there is no
separate `enabled` flag, matching how a missing block means off. `shift` defaults
to true. The field becomes typed on `Config` and gains a schema entry; the
untyped-preservation test already proves older files with this key survive.

Rejected: an `enabled: false` toggle that keeps the block. Absent-means-off is
simpler and the file is hand-edited, so commenting out the block is the natural
"off".

### One configured hyper set drives both emission and chord expansion

The hyperkey emits a modifier set, and the `hyper` token in a chord must expand to
the identical set, or `RegisterHotKey` (which matches modifiers exactly) never
fires: emit Ctrl+Alt+Super but register Ctrl+Alt+Shift+Super and the press is a
miss. So the set is computed once from the config, `Ctrl+Alt+Super` always plus
Shift unless `"shift": false`, and used in two places: the hook synthesizes
exactly it, and chord parsing expands `hyper` to it.

Concretely, `hotkey::parse` grows a variant that takes the hyper set; the free
`parse` keeps expanding `hyper` to the full four as the default (an unconfigured
`hyper` chord stays registrable), while the launcher and command-hotkey builders,
which already hold the `Config`, pass `config.hyper_modifiers()`. Only Shift is
optional for now; Ctrl, Alt, and Super are always in the set.

Rejected: a free-form modifier list in the config (`["ctrl","alt","super"]`).
Shift is the one that changes typed characters and the only exclusion asked for;
a list is more surface for no present need.

### Live reload rebuilds the service

On a config edit, `apply_config_reload` gains a step after the hotkeys: stop the
current hyperkey handle (which releases any synthesized modifiers) and start a
fresh one from the new config. A disabled or removed block stops it; a changed
key restarts it on the new key. This is the same drop-and-recreate the command
hotkeys use, so there is no diffing.

## Risks / Trade-offs

- **Stuck modifiers if a key-up is missed** (the hook is removed mid-hold, the
  session locks, focus is lost). → The handle's stop always synthesizes key-up
  for any modifier it holds, and stop runs on reload and on exit; held state is
  tracked so a released hyperkey never leaves modifiers down.
- **A slow callback is silently unhooked by Windows.** → The callback only tracks
  a bool and issues a fixed set of `SendInput` calls; no allocation, no blocking.
- **Games and raw-input / DirectInput apps bypass the hook, and elevated windows
  are out of reach.** → Accepted ceiling (proposal.md - Platforms); the key
  falls back to normal there rather than half-working.
- **The hyperkey modifies everything typed while held** (hyper+C is
  Ctrl+Alt+Shift+Win+C). → Intended; that is what a dedicated modifier is. Chords
  that are not bound simply do nothing, as any unbound combination does.
- **A real modifier held with the hyperkey** (Shift+hyperkey+key). → The
  synthesized modifiers are additive to whatever is physically down; no special
  handling needed.
- **macOS: the `hidutil` remap is machine-wide while it is set.** → It is
  applied on start and cleared on stop and on drop, and it does not survive a
  reboot, so the worst case after a crash is CapsLock staying F18 until Dango
  runs again or the machine restarts.
- **macOS 26 is reported not to deliver external-keyboard events to a session
  tap.** → Recorded as a ceiling to confirm during verification rather than
  designed around; the built-in keyboard is the supported case.

## Migration Plan

Additive. An absent `hyperkey` block is the default and changes nothing, so
existing configs and machines are unaffected. Rollback is removing the block (or
the feature); no stored state is migrated.
