# Design

## Context

See proposal.md - Why. Requirements are in this change's `specs/`.

Today, outcomes that happen while the launcher is hidden take three paths:

- The reducer (`src/ui/state.rs`) keeps a failure toast across `reset` so the
  next activation shows it. `Event::Failed` and `ActionOutcome::Failed` both
  end in `show_failure`.
- `Core::dispatch_command` and `Core::dispatch_application`
  (`src/app/mod.rs`) send hotkey problems to `report_problem`, which logs and
  turns the tray's config line to an error.
- `ActionOutcome::CopyToClipboard` emits `Effect::CopyText` and
  `Effect::Dismiss` and says nothing.

The launcher is one Slint window on winit 0.30 (Slint 1.18). Its native setup
sits behind `platform::LauncherWindow`. winit shows a window with
`SW_SHOWNOACTIVATE` only the first time; later shows use `SW_SHOW`, which
activates (`winit-0.30.13/src/platform_impl/windows/window_state.rs`).

## Goals / Non-Goals

**Goals:**

- One route for "tell the user while hidden", decided in the reducer so it is
  tested with `cargo test`.
- A HUD window that is built once and never activates on either platform.

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
  `Window::set_cursor_hittest(false)` (click-through on both platforms), and
  window level.

Rejected: reshaping the launcher window into the pill. The launcher holds
state during the grace period, has its own height and placement rules, and
takes the keyboard on show. Sharing one window would mix two lifecycles.

### D2. The reducer owns the HUD, as it owns the toast

The reducer learns whether the launcher is visible: `Event::Activated` sets
it, a new `Event::Hidden` clears it. `Core::hide` sends `Event::Hidden` for
every hide reason.

- `show_failure` while hidden sets `hud` instead of `toast`. The failure is no
  longer kept across `reset`.
- `CopyToClipboard` emits `CopyText`, `Dismiss`, and sets a success `hud`
  "Copied to clipboard". The launcher is still visible at that moment, but it
  is being dismissed, so the HUD shows after the hide.
- `Event::Activated` clears `hud`.
- Lifetime copies D7 of `component-system`: setting `hud` emits
  `Effect::ExpireHud { id, style }`. `Core` waits about two seconds for
  success, about four for failure, and returns `Event::HudExpired { id }`. A
  newer HUD has a new id, so a stale expiry does nothing.

`Snapshot` gains `hud: Option<Hud { style, text }>`. `ToastStyle` is reused;
`Busy` never reaches the HUD.

Hotkey paths join the same route: a `run_command` error from
`dispatch_command` becomes `Event::Failed`, and a missing application becomes
`Event::Failed` with "Can't open {name} because it isn't installed". Both
are still written to the log. They no longer call `report_problem`, so the
tray line stays for config and registration problems. The ambiguity note for
an application name stays in the tray and log: the application still opens.

Rejected: deciding in `Core` by `core.visible` before calling the reducer.
That splits one rule across two places and leaves the paste race (failure
arrives before or after the hide) untested.

### D3. Renderer writes a `Hud` global, the shell shows and hides the window

`src/slint_shell/render/` gets one more writer, `Hud` (`style`, `text`,
`visible`), in line with "one writer per global". When `snapshot.hud` changes
from none to some, or its id changes, `SlintShell` positions the window and
shows it; when it becomes none, it hides it. This runs where `publish` already
reaches the main thread.

`HudWindow` uses the existing `Dot` atom and `Theme` values: toast dot, toast
text style, raised surface, the launcher's radius, and a max width equal to
the ActionBar toast slot at the launcher's width. Its height is fixed, so the
window size is set by counting, like rows. If `Theme` lacks a value the pill
needs (pill padding), it is added there with dark and light values.

### D4. Native show without activation, behind `platform::HudWindow`

A narrow trait in `src/platform/mod.rs`, implemented in `platform/windows` and
`platform/macos`:

```
trait HudWindow {
    fn attach(&mut self, window: RawWindowHandle);   // once
    fn show(&self, window: RawWindowHandle, size: LogicalSize);
    fn hide(&self, window: RawWindowHandle);
}
```

Slint shows `HudWindow` once at startup, created with `with_active(false)`
through a `with_window_attributes_hook` on the backend selector in
`src/slint_shell/mod.rs`, matched on the window title. After
that, visibility is toggled natively, so winit's later `SW_SHOW` never runs:

- Windows: `attach` sets `WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST
  | WS_EX_TRANSPARENT | WS_EX_LAYERED`, removes `WS_EX_APPWINDOW`, and hides
  the window. `show` finds the monitor with `MonitorFromWindow` on
  `GetForegroundWindow` (falling back to the cursor's monitor), places the pill
  centred above the bottom of `rcWork` in physical pixels at that monitor's
  DPI, and calls `SetWindowPos(HWND_TOPMOST, SWP_NOACTIVATE | SWP_SHOWWINDOW)`.
- macOS: `attach` reclasses to a non-activating NSPanel as the launcher does,
  sets `ignoresMouseEvents`, a level above the launcher's, and collection
  behaviour `CanJoinAllSpaces | FullScreenAuxiliary | IgnoresCycle |
  Stationary`. `show` uses `NSScreen::mainScreen`'s `visibleFrame` and
  `orderFrontRegardless`; `hide` uses `orderOut`. No AX calls (see the
  system-wide AX trap in `AGENTS.md`).

Rejected: calling Slint's `show()` and `hide()` for every HUD. On Windows the
second show activates the window and takes the foreground away from the user,
which also breaks the paste flows.

### D5. Text

New strings, checked with the ux-writing skill against `interface-copy`:

| Where | Text |
|-------|------|
| HUD after a copy closes the launcher | "Copied to clipboard" |
| HUD for an application hotkey with no app | "Can't open {name} because it isn't installed" |

"Copied to clipboard" is taken from the design handover's toast example.
Every other HUD text is an existing failure sentence, unchanged.

### Contracts

No change to the extension manifest or the view protocol's wire format.
`ActionOutcome` is unchanged; only what the launcher does with
`CopyToClipboard` and `Failed` while hidden changes.

## Risks / Trade-offs

- [`WS_EX_NOACTIVATE` plus `SWP_SHOWWINDOW` still activates on some Windows
  build] → Hand check 4.1 watches the foreground window while a HUD shows.
  Fallback: `ShowWindow(SW_SHOWNOACTIVATE)` after placing.
- [Slint stops rendering a window it thinks is visible but the OS has hidden]
  → Slint redraws on property change for any window it has shown; hand check
  shows two HUDs with different text in a row.
- [`NSScreen::mainScreen` follows Dango's own key window, not the frontmost
  app's] → Hand check on two displays; fallback is the display under the
  mouse, as the launcher uses.
- [A HUD over the bottom of a fullscreen video or game] → Accepted: it is
  short-lived and click-through.
- [The attributes hook is global to every Slint window] → It matches on the
  HUD window's title only; a UI test checks the launcher still activates.
