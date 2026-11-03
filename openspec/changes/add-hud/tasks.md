# Tasks

## 1. HUD state in the reducer

- [ ] 1.1 Add `Hud { id, style, text }` to the reducer and `hud` to `Snapshot`, with `Effect::ExpireHud { id, style }` and `Event::HudExpired { id }` per design D2; `Event::Activated` clears it; verify reducer tests in `src/ui/tests.rs` for a stale expiry ignored, a newer HUD replacing an older one, and activation clearing the HUD
- [ ] 1.2 Make `CopyToClipboard` set a success HUD "Copied to clipboard" next to `CopyText` and `Dismiss`; verify a reducer test that the copy emits all three and leaves no toast
- [ ] 1.3 Run `ExpireHud` in `Core::run_effect` with about two seconds for success and about four for failure; verify a `Core` test with the recording shell and paused tokio time that the HUD is gone after each lifetime

## 2. HUD window

- [ ] 2.1 Add `ui/shell/hud.slint` (`HudWindow`) from `Dot` and `Theme` per design D3, exported from `ui/app.slint`, reading a new `Hud` global; add any missing pill value to `Theme` in dark and light; verify `mise run check` compiles the `.slint` files
- [ ] 2.2 Add the `Hud` writer in `src/slint_shell/render/` and a UI test in `src/slint_shell/tests.rs` that a failure HUD and a success HUD show their text, dot style, and accessible label, and that the longest failure string is not elided at the HUD's max width
- [ ] 2.3 Add `platform::HudWindow` with a no-op default, create `HudWindow` at startup with `with_active(false)` through a `with_window_attributes_hook` matched on its title, show it once, and let `SlintShell` call the trait when `snapshot.hud` appears, changes id, or goes; verify `cargo test` with a UI test that the launcher still takes focus state as before
- [ ] 2.4 Implement `HudWindow` on Windows in `platform/windows` per design D4 (styles, monitor of the foreground window, physical-pixel placement above `rcWork`'s bottom, `SWP_NOACTIVATE | SWP_SHOWWINDOW`, hide); verify `cargo clippy --all-targets -- -D warnings` on Windows
- [ ] 2.5 Implement `HudWindow` on macOS in `platform/macos` per design D4 (non-activating panel, `ignoresMouseEvents`, level, collection behaviour, `mainScreen` `visibleFrame`, `orderFrontRegardless`, `orderOut`); verify symbol names against the vendored `objc2-app-kit` source and a macOS compile in CI

## 3. Failures go to the HUD while hidden

- [ ] 3.1 Add `Event::Hidden`, sent by `Core::hide` for every hide reason; make `show_failure` set a failure HUD instead of a toast while hidden, and drop the failure kept across `reset`; verify reducer tests for a failure after a dismissing action, a failure during the grace period, and no failure toast on the next activation, and update the existing carried-failure tests
- [ ] 3.2 Route `dispatch_command`'s `run_command` error and `dispatch_application`'s missing application through `Event::Failed`, the latter with "Can't open {name} because it isn't installed", still logged, no longer through `report_problem`; verify `Core` tests with the recording shell that each shows a HUD and leaves the tray status unchanged
- [ ] 3.3 Add the platform trap from design D4 to `AGENTS.md`: winit shows with `SW_SHOW` after the first show, which activates, so the HUD toggles visibility natively; verify `mise run check`

## 4. Hand checks on Windows

- [ ] 4.1 With Notepad in front, press a window-management hotkey on an elevated window (`mise run keys`), and check with `mise run shot` that the HUD shows the failure at the bottom of that monitor, Notepad keeps the foreground and typing, and the HUD is gone after about four seconds
- [ ] 4.2 Copy from clipboard history in the launcher and check the "Copied to clipboard" HUD shows after the hide, clicks pass through it, and it is not in the taskbar or Alt+Tab
- [ ] 4.3 Show two HUDs in a row with different text and check the second replaces the first; open the launcher over a HUD and check the HUD is gone
- [ ] 4.4 Check the HUD over a fullscreen app and on a second monitor with the focused window there

## 5. Hand checks on macOS

- [ ] 5.1 With TextEdit in front, trigger a hotkey failure and check the HUD shows at the bottom of that display above the Dock, TextEdit stays active and keeps typing, and Dango does not appear in the Dock or Cmd+Tab
- [ ] 5.2 Check the HUD over a fullscreen app in its own Space, and on a second display holding the focused window
- [ ] 5.3 Copy from clipboard history and check the "Copied to clipboard" HUD shows and clicks pass through it

## 6. Review

- [ ] 6.1 Run the `ponytail-review` skill in a fresh subagent on the change's commits, without the apply context; record its findings for the author
