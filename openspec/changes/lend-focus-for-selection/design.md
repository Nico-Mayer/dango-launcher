# Design

## Context

See proposal.md for the problem. The current read path, in `TextExchange::selection`
(`src/text/mod.rs`):

```
permitted? -> refused app? -> [Windows: dismiss + yield_to_previous]
           -> direct read (UIA / AX) -> Text | Empty | Unavailable
           -> Unavailable: [dismiss + yield] -> sentinel -> copy chord -> wait 200ms
```

- Windows: the launcher holds the foreground, so UIA's "focused element" is
  Dango's search field. `WindowsSelection::needs_foreground` makes the exchange
  hide first. `yield_to_previous` calls `force_foreground(previous)` and waits
  until the previous window is really in front.
- The launcher is `WS_EX_TOPMOST` (`apply_tool_window_style`), so another
  window taking the foreground does not cover it.
- Focus loss on Windows is `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)` with
  `WINEVENT_SKIPOWNPROCESS`, calling `Core::hide_on_focus_loss`. Giving the
  foreground to another process fires it.
- macOS: the launcher is a non-activating `NSPanel`, the app behind stays
  active. `MacSelection` asks the app element, then the system-wide element.
  The system-wide focused element can be the panel's own search field, which
  answers `""`, so Dango reports Empty and never copies (inferred from code,
  matches the reported behaviour, not tested).

## Goals / Non-Goals

**Goals:**
- One read path for root search and command hotkeys, with no hide.
- Platform differences stay behind the existing `Handoff`, `Keys`,
  `DirectSelection` traits.

**Non-Goals:**
- Changing insertion (`insert`, `paste_content`, `expand`). They still hide.

## Decisions

### Lend the foreground instead of hiding

`Handoff` gets a lend step for reads:

```
selection():
  lend = handoff.lend_to_previous()?     // Windows: pause focus loss, force_foreground(previous), wait
  direct read
  if Unavailable: sentinel, copy chord, wait, restore clipboard
  drop(lend)                             // Windows: force_foreground(launcher), resume focus loss
```

The lend is a guard value so every exit path, errors included, gives the
foreground back. `needs_foreground` and `Launcher::dismiss` leave the read path;
`dismiss` stays for insertion.

- Windows: `lend_to_previous` reuses `yield_to_previous`'s force-and-wait. The
  return calls `force_foreground` on the launcher's hwnd, the same call
  `did_show` makes, which also gives Slint's search field the keyboard again.
- macOS: lend is a no-op. The panel stays key; the AX read needs no focus
  change, and the copy goes to a pid (next decision).

Alternatives rejected:
- **Hide, copy, reshow.** Visible flicker; the view state reset on hide-for-work
  would have to be undone. The author accepted it only as a last resort.
- **Read at every activation**, before Dango takes the foreground. Races the
  show (UIA can answer with Dango's own field), reads the selection on every
  open, and UIA queries can switch Chromium apps into accessibility mode on
  every open (assumed, not measured). Does not help apps that need the copy
  fallback anyway.
- **Read without the foreground**: `GetGUIThreadInfo.hwndFocus` finds the
  inner control only in plain Win32 apps, and UIA's `HasKeyboardFocus` drops
  with the foreground. `WM_COPY` works only for Win32 edit controls.

### Focus loss ignores a lent foreground (Windows)

A process-wide `LENDING` flag, set by the lend and cleared by its drop. The
foreground event handler hides only when not lending and when
`GetForegroundWindow()` at handling time is not Dango's own window. The second
check covers the event for the lend arriving late, after the foreground came
back: the handler runs on the main thread's message loop, later than the
foreground change.

### Copy chord: Ctrl+Insert on Windows

`WindowsKeys::copy` sends Ctrl+Insert through enigo (`Key::Insert`; enigo
0.6.1 sets `KEYEVENTF_EXTENDEDKEY` for `VK_INSERT`, checked in vendored
source), held with `CHORD_HOLD` like the paste chord. A late Ctrl would turn a
plain Insert into an overtype toggle, so the hold matters here too.

No chain to Ctrl+C on failure: that brings back SIGINT in terminals and editor
commands in modal keymaps, and doubles the wait.

Verified: Zed copies on Ctrl+Insert (by hand). Assumed: Windows Terminal,
Chromium, Electron, Office, Qt and GTK apps copy on it. Hand check covers a
browser and a terminal.

### macOS: copy posted to the pid

`MacKeys::copy` posts the Cmd+C events with `CGEvent::post_to_pid`
(`CGEventPostToPid`, in `objc2-core-graphics` 0.3 already a dependency with
the `CGEvent` feature) to the frontmost application, so the panel can stay
key. Unverified that every app handles events posted this way; the hand check
covers a native app, a browser and Zed. If it fails, fallback is to resign key
on the panel without hiding it, decided in that Mac session.

Mac hand check: TextEdit, Firefox and Zed all handle it. Fallback not needed.

### macOS: trust only real answers

`MacSelection::read`:
- asks only the frontmost application's element, never the system-wide one.
  While the panel is key, macOS answers a system-wide query (focused element
  or focused application) in-process on the reader thread, and accesskit
  panics on a `RefCell` the main thread holds, aborting Dango (found in the
  Mac hand check);
- skips any focused element whose pid is Dango's own (`AXUIElementGetPid`);
- treats `""` as Empty only if the element also has `AXSelectedTextRange`;
  otherwise Unavailable.

Keeps "an app that answered empty gets no keystroke" for apps that really
answer, and sends GPUI-style apps to the copy fallback.

### Drop the exclusion list

Removes `config::Selection`, `RefusesFallback`, `refuses_fallback`,
`TextError::SelectionRefused`, the `refuses` argument through
`platform::text_exchange`, schema example, `docs/config.md` row, and the macOS
walkthrough step. The config struct has no `deny_unknown_fields`, so an old
`selection` key is ignored; the editor schema flags it.

### Crates evaluated

`get-selected-text` (Windows: plain Ctrl+C round trip; macOS: AX then Cmd+C
only on error) and pot-app's `Selection` (UIA then Ctrl+C with a fixed wait,
text and image restore only). Neither handles GPUI apps nor keeps the launcher
up, and both do less than `TextExchange` (sentinel, full clipboard restore,
deadline). Not adopted.

No manifest or view protocol change. No new user-facing strings; one removed:
"Copy the text first, as {app} doesn't share its selection".

## Risks / Trade-offs

- [Foreground lock refuses the return to Dango] → the return uses the same
  `force_foreground` as `did_show`; on failure the launcher is still on screen
  and the user can click it. Logged.
- [The previous window flashes active for a moment] → topmost launcher stays on
  top; only title-bar tint changes. Accepted.
- [Ctrl+Insert unsupported in some app] → the read reports "Select some text
  first" there, as today for GPUI apps. No destructive side effect.
- [Editor copies the current line when nothing is selected] → Zed does this on
  Ctrl+Insert, as on Ctrl+C, so the read returns that line instead of
  "Select some text first". No chord avoids it. Accepted (Windows hand check).
- [`CGEventPostToPid` ignored by some macOS apps] → Mac hand check passed in
  TextEdit, Firefox, Zed; fallback plan above if another app ignores it.
- [Late foreground event hides the launcher after the lend] → handler re-checks
  the current foreground.

## Migration Plan

None. Remove `selection` from the author's config by hand.
