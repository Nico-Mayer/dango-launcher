# Design

## Context

`adopt-components` made AI requests background tasks. A Windows hand check
showed three rough edges:

- The success of a task that ends while the launcher is hidden is silent.
  `task_changed` (`src/ui/state.rs`) shows the HUD only for a failure, and
  keeps the result in `pending_result` for a toast on the next open.
- That toast, "Answer ready" with "Show answer", expires like any success toast
  (`TOAST_LIFETIME`, 2 s, in `src/app/mod.rs`).
- Chords have three formats on Windows. `panel_label` gives "Ctrl K",
  `confirm_label` gives "Ctrl↵", and `shortcut_label` and `hotkey_label` join
  with `Label::ChordSeparator` ("+" on Windows, "" on macOS). On macOS
  `panel_label` gives "⌘ K" while the rest give "⌘↵". The toast's `Kbd` in
  `ui/shell/action-bar.slint` also stretches wider than its text.

## Goals / Non-Goals

**Goals:** a success HUD for a hidden task result, a way back that does not
time out, a jump into the answer when the user opens during its HUD, and one
chord format per platform.

**Non-goals:** see the proposal. No view protocol or manifest change: shortcuts
stay `{ key, modifiers }`, only their drawing changes.

## Decisions

### A success HUD for a hidden task result

`task_changed`, when hidden, calls `show_hud(Success, sentence)` for `Done` and
for a newly arrived `NeedsInput`, as it does for `Failed`. `pending_result`
stays as it is, so the toast on the next open is unchanged.

`pending_result` gains the id of the HUD it showed (`hud: u64`), so the jump
below can tell that the HUD on screen is this result's and not a newer one,
such as a copy that replaced it.

### Opening during the HUD opens the answer

In `Event::Activated`, before `self.hud = None`: when the pending result has
`show`, and the HUD still showing has that result's id, the reducer pushes
`Effect::RunAction` with the same request "Show answer" carries
(`task_action`), and shows no toast. The AI extension's `show` takes the kept
answer and pushes its view through `runner.start()`, as the toast action does
today. Otherwise the pending result shows as a toast, as today.

A failure or a copy has no `show`, so it never jumps.

- **Rejected: a time window after the result** (open within N seconds and you
  land in it). The user cannot see the window. Tying the jump to the HUD makes
  it visible: if the pill is there, opening goes to it.
- **Rejected: an action on the HUD.** The HUD never takes focus or input
  (`hud` spec, and the platform traps for its non-activating window).

### A toast with an action stays

`show_success` takes the toast's action and pushes `ExpireToast` only when there
is none. `show_task_result` passes "Show answer" in, instead of attaching it
after the expiry was already scheduled. `clear_failure` becomes
`clear_lasting`: it clears a failure toast or a toast with an action, at every
place a failure toast is cleared today (key press, click, typing), so both kinds
end the same way.

- **Rejected: a longer timer** (6 s or 10 s). Any timer can still be missed, and
  a failure toast already sets the rule that a toast you may act on waits for
  you.

### A held answer is shown, not kept

An AI request that ends while its view is held behind a hide is still away,
but its view comes back on the next open. `adopt-components` already pushes the
answer into that view. Keeping it as well and offering "Show answer" would push
the same answer a second time. In `Inner::run`, the kept answer is stored only
when the view's context is cancelled (left, superseded, or hold expired), and
`finish_away` reports `Done` without `show` when the view is held. The HUD
still shows "Answer ready: <title>"; opening goes to the held view, which shows
the answer.

### One chord format

- `Label::ChordSeparator` becomes " " on Windows and stays "" on macOS.
- `panel_label` and `confirm_label` join with it: "Ctrl K" / "⌘K" and
  "Ctrl ↵" / "⌘↵". macOS's panel chord loses its space, matching the rest.
- `shortcut_label` and `hotkey_label` already join with it: "Ctrl ↵",
  "Ctrl .", "Ctrl R", "Ctrl Alt V".

- **Rejected: "Ctrl+K" everywhere.** The author chose the space, which is what
  the footer already shows.

### The toast's chord fits its text

The toast's chip sits in a `HorizontalLayout` with `horizontal-stretch: 0`,
yet takes extra width. The fix keeps the chip at its preferred width, as the
footer's chips are. The exact cause is found at apply time; the UI test pins
the result.

## Components

- **HUD:** the existing HUD window and its pill (`hud` spec). No change to how
  it draws.
- **Toast:** the ActionBar toast in `ui/shell/action-bar.slint`.
- **Key chord:** the `Kbd` atom in `ui/atoms/kbd.slint`, used by the footer,
  the action panel, the toast, and result rows.

No new component.

## Text

No new strings. These are reused:

- "Answer ready: <title>" and "Copied to clipboard", now also in the HUD.
- "Dango 0.5.0 is ready to restart" (the update's sentence), now also in the
  HUD.

## Risks / Trade-offs

- **An update's "ready to restart" HUD** now shows while hidden. It reports
  once per download, so it is not noise.
- **A toast with an action now stays.** "Retry" on a failure already did.
  Cancel on the busy toast is a busy toast, which follows the busy marks, not
  this rule.
- **The jump runs an action on open.** It only runs `show`, which pushes a view
  and changes nothing else, so an accidental open costs one Escape.
