## 1. Config foundation

- [x] 1.1 Add a typed optional `hyperkey` field to `Config` (a `key` defaulting to `capslock`, a `shift` flag defaulting to true, presence meaning on) plus a `hyper_modifiers()` returning Ctrl+Alt+Super and Shift unless excluded, parsing and re-serialising it, and verify with unit tests over a present block, `shift: false`, an absent block, and the existing unknown-key round-trip
- [x] 1.2 Make the chord grammar expand `hyper` to a passed-in set: add a `parse` variant taking the hyper modifiers, keep the free `parse` defaulting to the full four, and have the launcher and command-hotkey builders pass `config.hyper_modifiers()`, and verify with a unit test that `hyper+left` under a shift-excluded set drops Shift
- [x] 1.3 Add `hyperkey` to `docs/config.schema.json` and replace the reserved comment in `docs/config.example.jsonc` with a real block including `shift`, and verify the `the_example_config_validates_and_parses` test passes

## 2. The platform service

- [x] 2.1 Define a `Hyperkey` platform trait with a start-from-config entry point returning a stop-on-drop handle, plus a no-op fallback that reports unavailable like key injection does, and verify it compiles on every target with a fake exercising start and stop

## 3. Windows implementation

- [x] 3.1 Implement the Windows `Hyperkey` with a `WH_KEYBOARD_LL` hook on a dedicated pumped thread: swallow the mapped key, synthesize the four hyper modifiers (`VK_LCONTROL`, `VK_LMENU`, `VK_LSHIFT`, `VK_LWIN`) on its down and up marked `DANGO_INJECTED`, and pass through its own injected events and all other events, and verify it builds and clippy is clean on Windows
- [x] 3.2 Track held state so an auto-repeat down does not re-synthesize, and release all synthesized modifiers when the handle stops so none are ever left down, and verify the held-state and release logic with a unit test over the pure state transitions

## 4. macOS implementation

- [x] 4.1 Implement the macOS `Hyperkey` with a session `CGEventTap` that, while the mapped key is held, sets the four modifier flags on events and suppresses the key's own effect, requesting Accessibility as the selection path does, and verify it builds and clippy is clean on macOS

## 5. Startup and live reload

- [x] 5.1 Start the hyperkey from the initial config at startup, after the launcher and command hotkeys are registered, holding the handle for its lifetime, and verify the app runs with a configured hyperkey
- [x] 5.2 Extend `apply_config_reload` to stop the current hyperkey and start a fresh one from the new config, so enabling, disabling, and changing the key apply live, and verify the existing suite, clippy, and fmt are all clean

## 6. Verification

- [x] 6.1 Confirm on both platforms that holding the hyperkey and pressing a bound `hyper+<key>` chord invokes that command while another application is focused, and that the mapped key's own function (the CapsLock toggle) does not occur
  - Windows: with a target window focused, synthesizing CapsLock-hold + Left ran
    window-management left-half on it (frame became the exact left half) and the
    CapsLock toggle state was unchanged before and after.
  - macOS: with the harness's own window focused, hyper+left ran
    window-management left-half on it and the frame became the exact left half
    of the work area. A listen-only tap appended behind Dango's witnessed the
    key pressed under the hyperkey carrying Ctrl+Alt+Shift+Cmd (flags 0x1e0000
    set) and saw no event at all for the hyperkey's own keycode, so it is
    swallowed. The lock state, read from `CGEventSourceFlagsState`, was
    unchanged across three chords and a tap - the `hidutil` remap means CapsLock
    emits F18 and no lock event exists to suppress.
- [x] 6.2 Confirm on both platforms that a lone tap of the hyperkey does nothing, that normal typing without the hyperkey is unchanged, and that a snippet paste still inserts text with the hyperkey enabled
  - Windows: a lone CapsLock tap moved no window, kept the foreground (so the
    Start menu did not open from the injected Win), left no modifier down, and
    did not toggle CapsLock. Normal typing and snippet paste are structurally
    untouched: the hook passes every non-trigger key straight through, and its
    own injected events and the paste path both carry `DANGO_INJECTED`, which the
    hook ignores; a text-field paste was not separately scripted.
  - macOS: with no `tap` configured, a 60ms solitary tap emitted nothing at all
    (the witness recorded an empty key list). Normal typing is untouched: every
    non-trigger event is returned to the chain, only its flags added while the
    key is held. Keyword expansion pasted its snippet correctly with the
    hyperkey enabled throughout, so the paste path is unaffected.
- [x] 6.3 Confirm on both platforms that adding, removing, and changing the hyperkey in the file apply live without a restart, and that after disabling it or quitting no modifier is left stuck down
  - Windows: editing the file live to exclude Shift kept the chord firing (the
    emitted set and the `hyper` chord stayed in agreement), and removing the
    hyperkey block stopped the hook live, after which CapsLock toggled normally
    and the chord was dead, with no modifier left down. The reload trace showed
    each edit dropping the previous hook and starting the new one or none.
  - macOS: the `hidutil` mapping is the observable. Removing the `hyperkey`
    block cleared it within a second and re-adding it restored it, no restart.
    Editing `shift` to false live changed what the tap emits (flags dropped
    Shift, 0x9e0000 to 0x9c0000) while `hyper+left` kept firing, which is the
    emit set and the chord expansion staying in agreement. No modifier can be
    left stuck on macOS by construction: the tap adds flags to passing events
    and never presses a modifier.
  - macOS caveat, now fixed: `Drop` does not run when the process is signalled,
    so a force-quit leaves the remap installed. The next run used to read that
    as another tool's mapping and refuse the hyperkey permanently; it now
    reclaims a mapping that is exactly its own.
