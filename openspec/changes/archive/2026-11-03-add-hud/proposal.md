# Proposal

## Why

When the launcher is not on screen, Dango has no way to tell the user anything.
A window-management hotkey that fails, a paste that is refused after the
launcher hid, or a copy that closes the launcher all end in silence. A failure
waits for the next time the launcher opens, often next to an unrelated task.
The design handover planned for this ("For actions that close the window, use a
HUD instead - a small pill shown briefly after hide"), and `component-system`
left it for its own change.

## What Changes

- A HUD: a small pill window with a dot and one line of text, shown briefly at
  the bottom of the screen under the mouse pointer while the launcher is
  hidden. It never takes focus and ignores the mouse.
- The HUD only reports outcomes the user cannot see otherwise:
  - a failure that happens while the launcher is hidden (a no-view command run
    from its hotkey, an action that hid the launcher and then failed, an
    application hotkey whose application is missing or fails to open)
  - a copy that closes the launcher, as "Copied to clipboard"
- Outcomes the user can already see, such as a moved window or an opened app,
  show nothing.
- A success HUD stays about two seconds, a failure HUD about four. A new HUD
  replaces the one showing. Opening the launcher removes the HUD.
- **BREAKING** (behaviour): a failure that happens while the launcher is hidden
  is no longer kept and shown on the next activation. The HUD shows it at once.
- A press-time failure of an application hotkey moves from the tray status
  line to the HUD. Registration problems (refused chord, collision) stay in the
  tray.

Platforms: Windows and macOS, both fully covered. Each needs its own native
window setup for a window that never activates and shows over fullscreen apps,
so each gets its own hand check.

## Non-goals

- AI requests that keep running after the launcher hides, and a "finished"
  HUD for them. Today a hide cancels the request (`ai-commands`); changing that
  is its own change.
- Success HUDs for outcomes the user can see: moved windows, launched apps,
  locked screen, emptied trash.
- Actions, buttons, or clicks on the HUD.
- Waiting for a key press to dismiss a failure HUD. That needs a global key
  hook.
- The design's animated toast style, and any enter or exit animation.
- OS notifications (Action Center, Notification Center).
- A setting to turn the HUD off or move it.
- A HUD shown while the launcher is visible. The ActionBar toast keeps that
  job.

## Capabilities

### New Capabilities

- `hud`: the pill shown while the launcher is hidden - when it shows, what it
  says, how long it stays, where it sits, and that it never takes focus or
  input.

### Modified Capabilities

- `launcher-shell`: "Reset to root state on hide" loses the exception that
  keeps an unseen failure for the next activation.
- `view-protocol`: "No-view commands report their outcome" sends a failure to
  the HUD when the launcher is hidden.
- `command-hotkeys`: a missing application or a failed launch on a hotkey
  press is reported in the HUD and the log instead of the tray status line.

## Impact

- `src/ui/` reducer: the failure kept across reset goes away.
- `src/app/`: `Core` decides between toast and HUD by whether the launcher is
  visible; hotkey failure paths stop calling `report_problem`.
- `src/shell.rs`: `Shell` gains a HUD call, implemented by `src/slint_shell/`
  and the recording fake.
- `ui/`: a new HUD window component exported from `app.slint`, built from the
  existing `Dot` atom and `Theme`.
- `src/platform/windows` and `src/platform/macos`: placement and the native
  setup winit does not cover (tool window style, non-activating `NSPanel`),
  behind a narrow trait.
- No change to the extension manifest or the view protocol's wire format.
