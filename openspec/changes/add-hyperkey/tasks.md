## 1. Config foundation

- [x] 1.1 Add a typed optional `hyperkey` field to `Config` (a `key` defaulting to `capslock`, a `shift` flag defaulting to true, presence meaning on) plus a `hyper_modifiers()` returning Ctrl+Alt+Super and Shift unless excluded, parsing and re-serialising it, and verify with unit tests over a present block, `shift: false`, an absent block, and the existing unknown-key round-trip
- [x] 1.2 Make the chord grammar expand `hyper` to a passed-in set: add a `parse` variant taking the hyper modifiers, keep the free `parse` defaulting to the full four, and have the launcher and command-hotkey builders pass `config.hyper_modifiers()`, and verify with a unit test that `hyper+left` under a shift-excluded set drops Shift
- [x] 1.3 Add `hyperkey` to `docs/config.schema.json` and replace the reserved comment in `docs/config.example.jsonc` with a real block including `shift`, and verify the `the_example_config_validates_and_parses` test passes

## 2. The platform service

- [ ] 2.1 Define a `Hyperkey` platform trait with a start-from-config entry point returning a stop-on-drop handle, plus a no-op fallback that reports unavailable like key injection does, and verify it compiles on every target with a fake exercising start and stop

## 3. Windows implementation

- [ ] 3.1 Implement the Windows `Hyperkey` with a `WH_KEYBOARD_LL` hook on a dedicated pumped thread: swallow the mapped key, synthesize the four hyper modifiers (`VK_LCONTROL`, `VK_LMENU`, `VK_LSHIFT`, `VK_LWIN`) on its down and up marked `DANGO_INJECTED`, and pass through its own injected events and all other events, and verify it builds and clippy is clean on Windows
- [ ] 3.2 Track held state so an auto-repeat down does not re-synthesize, and release all synthesized modifiers when the handle stops so none are ever left down, and verify the held-state and release logic with a unit test over the pure state transitions

## 4. macOS implementation

- [ ] 4.1 Implement the macOS `Hyperkey` with a session `CGEventTap` that, while the mapped key is held, sets the four modifier flags on events and suppresses the key's own effect, requesting Accessibility as the selection path does, and verify it builds and clippy is clean on macOS

## 5. Startup and live reload

- [ ] 5.1 Start the hyperkey from the initial config at startup, after the launcher and command hotkeys are registered, holding the handle for its lifetime, and verify the app runs with a configured hyperkey
- [ ] 5.2 Extend `apply_config_reload` to stop the current hyperkey and start a fresh one from the new config, so enabling, disabling, and changing the key apply live, and verify the existing suite, clippy, and fmt are all clean

## 6. Verification

- [ ] 6.1 Confirm on both platforms that holding the hyperkey and pressing a bound `hyper+<key>` chord invokes that command while another application is focused, and that the mapped key's own function (the CapsLock toggle) does not occur
- [ ] 6.2 Confirm on both platforms that a lone tap of the hyperkey does nothing, that normal typing without the hyperkey is unchanged, and that a snippet paste still inserts text with the hyperkey enabled
- [ ] 6.3 Confirm on both platforms that adding, removing, and changing the hyperkey in the file apply live without a restart, and that after disabling it or quitting no modifier is left stuck down
