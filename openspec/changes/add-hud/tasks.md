# Tasks

## 1. HUD state in the reducer

- [x] 1.1 Add `Event::Hidden`, sent from `Core::settle_hide` so every hide path reports it, and track visibility in the reducer with `Event::Activated`; verify a `Core` test with the recording shell that `hide`, `hide_after_launch`, `hide_on_focus_loss`, and `hide_for_work` each reach the reducer as hidden
- [x] 1.2 Add `Hud { id, style, text }` to the reducer and `hud` to `Snapshot`, exposed only while hidden; give `Effect::ExpireToast` the style and use it and `Event::ToastExpired` for the HUD too, from the one id counter, emitted when the HUD becomes visible, per design D2; `Event::Activated` clears it; verify reducer tests in `src/ui/tests.rs` for a stale expiry ignored, a newer HUD replacing an older one, activation clearing the HUD, and a HUD set while visible appearing only after `Event::Hidden`
- [x] 1.3 Make `CopyToClipboard` set a pending success HUD "Copied to clipboard" next to `CopyText` and `Dismiss`; verify a reducer test that the copy emits both effects, leaves no toast, and shows the HUD after `Event::Hidden`
- [x] 1.4 Wait `TOAST_LIFETIME` for success and a new `FAILURE_HUD_LIFETIME` of about four seconds for a failure in `Core::run_effect`; verify a `Core` test with the recording shell and paused tokio time that each HUD is gone after its lifetime

## 2. HUD window

- [x] 2.1 Add `ui/shell/hud.slint` (`HudWindow`) from `Dot` and `Theme` per design D3, exported from `ui/app.slint`, with its own `style` and `text` properties; add any missing pill value to `Theme`; verify `mise run check` compiles the `.slint` files
- [x] 2.2 Add the HUD writer in `src/slint_shell/render/` and a UI test in `src/slint_shell/tests.rs` that a failure HUD and a success HUD show their text, dot style, and accessible label, and that the longest failure string is not elided at the HUD's max width
- [x] 2.3 Add `platform::HudWindow` (`attach`, `place`, `did_show`), create `HudWindow` at startup with `with_active(false)` through a `with_winit_window_attributes_hook` gated by a flag set around its creation, call `set_cursor_hittest(false)` after each show, and let `SlintShell` place and `show()` it when `snapshot.hud` appears or changes id and `hide()` it when it goes; verify `cargo test` with a unit test of the show and hide decision and a UI test that the shell shows, rewrites, and hides the HUD window
- [x] 2.4 Implement `HudWindow` on Windows in `platform/windows` per design D4 (monitor under the cursor, physical-pixel placement above `rcWork`'s bottom with `SWP_NOACTIVATE`, `WS_EX_TOOLWINDOW` applied again after each show); verify `cargo clippy --all-targets -- -D warnings` on Windows
- [x] 2.5 Implement `HudWindow` on macOS in `platform/macos` per design D4 (reclass to plain `NSPanel`, not `DangoPanel`; non-activating mask, level above the launcher, collection behaviour, `visibleFrame` of the screen under the mouse); verify symbol names against the vendored `objc2-app-kit` source and a macOS compile in CI

## 3. Failures go to the HUD while hidden

- [x] 3.1 Make `show_failure` set a failure HUD instead of a toast while hidden, and drop the failure kept across `reset`; verify reducer tests for a failure after a dismissing action, a failure during the grace period, and no failure toast on the next activation, and update the existing carried-failure tests
- [x] 3.2 Route through `Event::Failed`, still logged and no longer through `report_problem`: `dispatch_command`'s `run_command` error, and from `dispatch_application` the missing application ("Can't open {name} because it isn't installed"), the failed launch (its own sentence), and the missing extension ("Can't open {name}"); verify `Core` tests with the recording shell that each shows a HUD and leaves the tray status unchanged
- [x] 3.3 Add to `AGENTS.md` platform traps: winit's `with_active(false)` holds for every show, not only the first; the HUD is a plain `NSPanel` because `DangoPanel` can become key and writes the global `PANEL_IS_KEY`; verify `mise run check`

## 4. Hand checks on Windows

- [ ] 4.1 With Notepad in front, press a window-management hotkey on an elevated window (`mise run keys`), and check with `mise run shot` that the HUD shows the failure at the bottom of the monitor under the mouse, Notepad keeps the foreground and typing, and the HUD is gone after about four seconds
- [x] 4.2 Copy from clipboard history in the launcher and check the "Copied to clipboard" HUD shows only after the launcher is gone, clicks pass through it, it is not in the taskbar or Alt+Tab, and the launcher still takes the keyboard on its next show
- [ ] 4.3 Show two HUDs in a row with different text and check the second replaces the first; open the launcher over a HUD and check the HUD is gone
- [ ] 4.4 Check the HUD over a fullscreen app, and on a second monitor with the mouse there

## 5. Hand checks on macOS

- [ ] 5.1 With TextEdit in front, trigger a hotkey failure and check the HUD shows at the bottom of the display under the mouse above the Dock, TextEdit stays active and keeps typing, and Dango does not appear in the Dock or Cmd+Tab
  - Done by window list and typing on one display: placed bottom centre, TextEdit stays active and gets every key, accessory policy. Left: see that the pill draws, and placement above a bottom Dock (Dock was right, autohide)
- [ ] 5.2 Check the HUD over a fullscreen app in its own Space, and on a second display with the mouse there
  - Fullscreen done: on screen at level 102 in TextEdit's fullscreen Space, typing continues. Left: see that it draws there, and a second display
- [ ] 5.3 Copy from clipboard history and check the "Copied to clipboard" HUD shows only after the launcher is gone, clicks pass through it, and the launcher still takes the keyboard on its next show
  - Done by window list, MCP text, and a real CGEvent click and keys: shows after the launcher is gone, click reaches TextEdit, launcher takes typing next show. Left: see that the pill draws

## 6. Review

- [x] 6.1 Run the `ponytail-review` skill in a fresh subagent on the change's commits, without the apply context; record its findings for the author
