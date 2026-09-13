## 1. Config

- [x] 1.1 Add an optional `tap` field to the hyperkey config (a key name in the launcher grammar, absent means off), parsed and re-serialised, and verify with unit tests over a present and an absent value plus a round trip
- [x] 1.2 Add `tap` to `docs/config.schema.json` and the example config with `"tap": "escape"`, and verify the example-config test passes

## 2. The Windows tap behavior

- [x] 2.1 Carry the tap key into the Windows hyperkey spec and record, on each fresh press, the press instant and an other-key-seen flag, setting the flag when any non-injected key other than the trigger goes down while the hyperkey is held, and verify it builds and clippy is clean
- [x] 2.2 On release, send the tap key (through the marked `SendInput` path, after the modifiers are released) only when the hold was under the threshold and no other key was seen, and verify the tap-versus-hold decision with a unit test over the pure decision (short and solitary sends; long, or other-key, does not)

## 3. Verification

- [x] 3.1 Confirm on both platforms that a quick solitary tap of the hyperkey sends the configured tap key, that a hold past the threshold sends nothing, and that a `hyper+<key>` chord still works and sends no tap key
  - Windows: a clean single-instance run traced a quick CapsLock tap (73ms, no
    other key) as `trigger down -> trigger up, other_key=false -> maybe_send_tap
    decided=true` and emitted Escape. The hold and chord non-tap decisions are
    unit tested (`should_tap`: long, or other-key, returns false), and `hyper+X`
    chords were verified live in add-hyperkey. macOS pending.
  - Note: an automated form-based harness was defeated by orphaned low-level
    hooks left by force-killed test instances (synthesized CapsLock arrived as
    vk=0) until a settle wait cleared them; the clean-room run then showed correct
    vk=20 delivery and the tap firing.
  - macOS: all three were driven and witnessed by a listen-only tap behind
    Dango's. A 60ms solitary tap emitted Escape (keycode 53); a 400ms hold
    emitted nothing; a chord, which is well under the threshold but not
    solitary, emitted only the chord key and no Escape. `hyper+left` kept
    running its command throughout.
- [x] 3.2 Confirm on both platforms that with no tap configured a tap does nothing, and that the tap key does not open the Start menu or leave a modifier stuck
  - Windows: with no tap configured the tap key resolves to 0 and the release
    path returns before sending anything (traced: `tap=0`). The Start-menu
    ordering (Win pressed first, released last) and the modifier release on the
    key's up are unchanged from add-hyperkey's verified behavior, so a tap leaves
    no modifier stuck.
  - macOS: with the `tap` field removed from the config live, the same 60ms tap
    emitted nothing. There is no Start-menu equivalent to guard against, and no
    modifier can be left stuck: the macOS hyperkey adds flags to events passing
    through rather than pressing modifiers, so there is nothing to release.
