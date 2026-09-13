## Why

Milestone: M5 (keys).

The launcher and per-command hotkeys already accept `hyper+X` chords, but no key
on the keyboard emits the hyper modifier, so those chords are unreachable. A
hyperkey turns one otherwise-wasted physical key (CapsLock by default) into a
dedicated Ctrl+Alt+Shift+Super modifier, the way Raycast and Hyperkey do, giving
a large collision-free chord space for launcher and command bindings. It is
configured in the same `config.json` that already carries the bindings, so a
hyperkey setup travels in the dotfiles with everything else.

## What Changes

- Add an optional top-level `hyperkey` block to the config: a physical key to
  remap (default `capslock`) whose press and release emit the hyper modifiers, so
  held-key chords like `hyper+left` fire. Absent means off.
- Make Shift optional in the combination, the way Raycast does: `shift` defaults
  to on (Ctrl+Alt+Shift+Super), and `"shift": false` drops it to Ctrl+Alt+Super,
  which avoids Shift changing the base character on letter chords. The same set
  drives both what the key emits and what the `hyper` token expands to, so the
  two can never disagree.
- Add a platform `Hyperkey` service that installs a system-wide key interceptor:
  while the mapped key is held it holds Ctrl+Alt+Shift+Super down, on release it
  lets them up, and it suppresses the key's normal function (the CapsLock
  toggle). Its own injected modifier events are marked and passed through, using
  the injection marker the text path already stamps.
- Start it at launch from the config and re-apply it live when the file changes
  (enable, disable, or change the key), alongside the launcher and command
  hotkeys.
- The hyperkey only produces the modifier combination; it registers no chords
  and decides no actions. It composes with the existing launcher and
  command-hotkey registration unchanged.

## Capabilities

### New Capabilities

- `hyperkey`: A configured physical key acts system-wide as the hyper modifier
  (Ctrl+Alt+Shift+Super), applied at startup and live on a config change, so
  `hyper+X` chords are reachable from one key.

### Modified Capabilities

- None at the requirement level for existing capabilities. Chord parsing gains a
  configured `hyper` set (see design.md), but registration is unchanged.

## Impact

- New code: a `Hyperkey` platform trait with a Windows implementation
  (`WH_KEYBOARD_LL` low-level keyboard hook on a dedicated pumped thread) and a
  macOS implementation (a `CGEventTap` rewriting the mapped key into modifier
  flag changes), plus a no-op fallback. New typed `hyperkey` config field and a
  schema entry (today the key is only preserved untyped).
- Touches the chord grammar so `hyper` expands to the configured set rather than
  a hardcoded four, config parsing, the schema and example docs, and the startup
  and reload paths in `lib.rs`.
- Reuses the `DANGO_INJECTED` marker so the hook ignores Dango's own output.

## Non-goals

- No settings or key-recorder GUI. The config file is the only surface.
- No tap-to-act behavior (a lone CapsLock tap sending Escape or a click); the
  key is purely the hyper modifier. Reserved for a later change.
- No per-application enable or disable, and no remapping of arbitrary keys to
  arbitrary output; this is one key to the one hyper combination.
- Not depending on an external tool such as PowerToys Keyboard Manager: it would
  not travel in the dotfiles and is not cross-platform.
- Does not overcome the platform ceilings below.

## Platforms

- Windows and macOS are both covered by the design. Windows is implemented and
  verified this milestone; macOS implementation follows and its runtime
  verification is deferred, matching the project's established split.
- Accepted ceilings (confirmed): on Windows a low-level hook does not see input
  destined for higher-integrity (elevated) windows, and games or applications
  reading raw input or DirectInput can bypass it; on macOS the event tap needs
  Accessibility permission and secure input fields will not receive the modifier.
