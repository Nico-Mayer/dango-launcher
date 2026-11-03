# Design

## Context

See proposal.md - Why. Requirements are in this change's `specs/`.

Today, outcomes that happen while the launcher is hidden take three paths:

- The reducer (`src/ui/state.rs`) keeps a failure toast across `reset` so the
  next activation shows it. `Event::Failed` and `ActionOutcome::Failed` both
  end in `show_failure`.
- `Core::dispatch_command` and `Core::dispatch_application`
  (`src/app/mod.rs`) send hotkey problems to `report_problem`, which logs and
  turns the tray's config line to an error. For applications that is a
  missing app, a failed launch, and a missing applications extension.
- `ActionOutcome::CopyToClipboard` emits `Effect::CopyText` and
  `Effect::Dismiss` and says nothing.

The launcher is one Slint window on winit 0.30 (Slint 1.18). Its native setup
sits behind `platform::LauncherWindow`. Every hide goes through
`Core::settle_hide`: `hide`, `hide_after_launch`, `hide_on_focus_loss`, and
`hide_for_work` all call it.

winit decides activation per show from the window's `active` attribute: with
`with_active(false)` every show uses `SW_SHOWNOACTIVATE`
(`winit-0.30.13/src/platform_impl/windows/window_state.rs`; `apply_diff` works
on a copy of the flags). winit rewrites the extended style on every visibility
change, so a style it does not manage, such as `WS_EX_TOOLWINDOW`, has to be
applied again after each show.

## Goals / Non-Goals

**Goals:**

- One route for "tell the user while hidden", decided in the reducer so it is
  tested with `cargo test`.
- A HUD window that is built once, shown and hidden through Slint, and never
  activates on either platform.

**Non-Goals:**

- A queue or history of HUD messages. One at a time.
- Sharing positioning code between platforms (`AGENTS.md`: physical pixels on
  Windows, logical points on macOS).

## Decisions

### D1. Own Slint window, not OS notifications

The HUD is a second Slint window, `HudWindow`, exported from `ui/app.slint`.

Crates evaluated for the job:

- `notify-rust` (Windows through `tauri-winrt-notification`, macOS through
  `mac-notification-sys`), `user-notify`: native toasts. They can't show
  within a second every time: Focus Assist and Do Not Disturb hide them, they
  pile up in Action Center and Notification Center, macOS asks for permission,
  and an unpackaged Windows app needs an AppUserModelID registered with a
  Start Menu shortcut. None can draw our pill or follow `Theme`.
- No Slint or winit crate offers a HUD or overlay window. winit itself gives
  the pieces we need: `WindowAttributes::with_active(false)`,
  `WindowLevel::AlwaysOnTop`, and `Window::set_cursor_hittest(false)`
  (click-through on both platforms).

Rejected: reshaping the launcher window into the pill. The launcher holds
state during the grace period, has its own height and placement rules, and
takes the keyboard on show. Sharing one window would mix two lifecycles.

### D2. The reducer owns the HUD, as it owns the toast

The reducer learns whether the launcher is visible: `Event::Activated` sets
it, a new `Event::Hidden` clears it. `Core::settle_hide` sends
`Event::Hidden`, so every hide path reports it, the paste hide included.

- `show_failure` while hidden sets `hud` instead of `toast`. The failure is no
  longer kept across `reset`.
- `CopyToClipboard` emits `CopyText` and `Dismiss` and sets a pending success
  HUD "Copied to clipboard".
- The snapshot exposes `hud` only while the reducer sees the launcher hidden.
  A HUD set while the launcher is still on screen (the copy) waits for
  `Event::Hidden`, so the HUD and the launcher are never on screen together.
- `Event::Activated` clears `hud`.
- Lifetime reuses D7 of `component-system`: `Effect::ExpireToast` gains the
  style and serves both toast and HUD, from the one id counter, answered by the
  existing `Event::ToastExpired`. For a HUD the effect is emitted when the HUD
  becomes visible. `Core` waits `TOAST_LIFETIME` (two seconds) for success and
  a new `FAILURE_HUD_LIFETIME` (four seconds) for failure. A newer toast or
  HUD has a new id, so a stale expiry does nothing.

`Snapshot` gains `hud: Option<Hud { style, text }>`. `ToastStyle` is reused;
`Busy` never reaches the HUD.

Hotkey paths join the same route through `Event::Failed`:

- a `run_command` error from `dispatch_command`
- a missing application, with "Can't open {name} because it isn't installed"
- a failed launch from `dispatch_application`, with the launch's own failure
  sentence
- a missing applications extension, with "Turn on Applications to use app
  hotkeys"

All are still written to the log. They no longer call `report_problem`, so the
tray line stays for config and registration problems. The ambiguity note for
an application name stays in the tray and log: the application still opens.

Rejected: deciding in `Core` by `core.visible` before calling the reducer.
That splits one rule across two places and leaves the paste race (failure
arrives before or after the hide) untested.

Rejected: a separate `ExpireHud` effect and `HudExpired` event. They would
copy the toast's timer one for one.

### D3. Renderer writes a `Hud` global, the shell shows and hides the window

`src/slint_shell/render/` gets one more writer, `Hud` (`style`, `text`), in
line with "one writer per global". When `snapshot.hud` changes from none to
some, or its id changes, `SlintShell` places the window and calls Slint's
`show()`; when it becomes none, it calls `hide()`. This runs where `publish`
already reaches the main thread.

`HudWindow` uses the existing `Dot` atom and `Theme` values: toast dot, toast
text style, raised surface, the launcher's radius, and a max width equal to
the ActionBar toast slot at the launcher's width. Its height is fixed, so the
window size is set by counting, like rows. If `Theme` lacks a value the pill
needs (pill padding), it is added there with dark and light values.

### D4. Slint shows and hides, platform code sets the window up

A `with_window_attributes_hook` on the backend selector in
`src/slint_shell/mod.rs`, matched on the HUD window's title, sets
`with_active(false)` and `WindowLevel::AlwaysOnTop`. winit keeps both across
shows. Once the window exists, `set_cursor_hittest(false)` through
`WinitWindowAccessor` makes it click-through; winit sets and keeps the styles
that needs.

The rest sits behind a narrow trait in `src/platform/mod.rs`, implemented in
`platform/windows` and `platform/macos`:

```
trait HudWindow {
    fn attach(&mut self, window: RawWindowHandle);              // once
    fn place(&self, window: RawWindowHandle, size: LogicalSize); // before show
    fn did_show(&self, window: RawWindowHandle);
}
```

Placement uses the display under the cursor on both platforms, the same signal
`position_on_active_display` uses for the launcher. It needs no AX call or
window list on macOS.

- Windows: `place` finds the monitor under the cursor and centres the pill
  above the bottom of `rcWork`, in physical pixels at that monitor's DPI, with
  `SWP_NOACTIVATE`. `did_show` applies `WS_EX_TOOLWINDOW` and removes
  `WS_EX_APPWINDOW` again, because winit rewrote the style on show.
- macOS: `attach` reclasses the window to plain `NSPanel`, not the launcher's
  `DangoPanel`: that class answers yes to `canBecomeKeyWindow` and writes the
  global `PANEL_IS_KEY` in `becomeKeyWindow`. A borderless `NSPanel` already
  answers no. `attach` also sets the non-activating style mask, a level above
  the launcher's, and collection behaviour `CanJoinAllSpaces |
  FullScreenAuxiliary | IgnoresCycle | Stationary`. `place` uses the
  `visibleFrame` of the screen under the mouse. No AX calls (see the
  system-wide AX trap in `AGENTS.md`).

Rejected: toggling visibility natively (`SWP_SHOWWINDOW`,
`orderFrontRegardless`) behind Slint's back. `with_active(false)` already
gives a show that does not activate, and native toggling would leave Slint
believing a hidden window is visible.

Rejected: the display holding the focused window. On macOS that needs AX or a
window list, and `NSScreen::mainScreen` could not be shown to follow another
app's window.

Rejected: setting `WS_EX_TRANSPARENT | WS_EX_LAYERED` by hand. A layered
window stays invisible until `SetLayeredWindowAttributes` is called, and
winit's `set_cursor_hittest` already handles it.

### D5. Text

New strings, checked with the ux-writing skill against `interface-copy`:

| Where | Text |
|-------|------|
| HUD after a copy closes the launcher | "Copied to clipboard" |
| HUD for an application hotkey with no app | "Can't open {name} because it isn't installed" |
| HUD for an application hotkey with the Applications extension off | "Turn on Applications to use app hotkeys" |

"Copied to clipboard" is taken from the design handover's toast example.
Every other HUD text is an existing failure sentence, unchanged.

### Contracts

No change to the extension manifest or the view protocol's wire format.
`ActionOutcome` is unchanged; only what the launcher does with
`CopyToClipboard` and `Failed` while hidden changes.

## Risks / Trade-offs

- [`with_active(false)` still activates the app on show on macOS] → Hand
  check 5.1 watches the active app while a HUD shows. The non-activating panel
  mask is the second guard.
- [The display under the cursor is not the one the user looks at, after a
  hotkey with the mouse parked on another display] → Accepted: the launcher
  uses the same rule, and the HUD is short-lived.
- [A HUD over the bottom of a fullscreen video or game] → Accepted: it is
  short-lived and click-through.
- [The attributes hook applies to every Slint window] → It matches on the HUD
  window's title only; hand check 4.2 checks the launcher still takes the
  keyboard.
