# Proposal

## Why

An AI command that finishes in the background is easy to miss, and hard to get
back to:

- **Nothing shows when it finishes.** While the launcher is hidden, only a
  failure shows in the HUD. A finished answer waits silently for the next open.
- **The way back is gone in two seconds.** "Answer ready" with "Show answer"
  is a success toast, so it disappears after about two seconds.
- **The key chord looks odd.** The toast draws "Ctrl+↵" in a chip much wider
  than its text, while the footer draws "Ctrl K" and "Ctrl↵". Windows has three
  chord formats on one bar.

## What Changes

- **A success HUD for a task result while hidden.** A task that finishes or
  needs input while the launcher is hidden shows a success HUD at once, as a
  failure already does. The toast on the next open stays.
- **Opening during that HUD lands in the result.** When the HUD reports a
  result that offers "Show answer" and the user opens the launcher before it
  is gone, with the launcher hotkey or the tray menu, the launcher opens on the
  answer itself, with no toast. A command that shows the launcher never jumps.
- **A toast with an action stays.** A success toast that carries an action
  stays until the next key press or click, as a failure toast does. A success
  toast without an action still goes after about two seconds.
- **No toast over a held answer.** When the launcher reopens onto the answer's
  own view, held behind a hide, that view is the result: no toast and no jump.
- **One chord format.** Every key chord is drawn as its keys with a space
  between them on Windows ("Ctrl K", "Ctrl ↵", "Ctrl Alt V") and joined with
  no space on macOS ("⌘K", "⌘↵", "⌃⌥V"), as macOS does today.
- **The toast's chord fits its text**, like every other key chord.

## Non-goals

- No change to how long a HUD shows: about two seconds for success, four for a
  failure.
- No jump for a failure or a copy: they have no view to open. A failure keeps
  its HUD and toast.
- No change to results that arrive while the launcher is shown.
- No action on the HUD itself: it still never takes focus or input.

## Platforms

Windows and macOS. The chord format differs by platform as described, with a
test per platform. The HUD, the toast, and the jump are platform-neutral
reducer behaviour; the HUD window already exists on both.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `hud`: a task's result while hidden is an outcome the user cannot see, so a
  success shows too; opening the launcher during a HUD that offers "Show
  answer" opens the answer.
- `background-tasks`: a success while hidden shows in the HUD at once, and
  opening during that HUD lands in the result.
- `design-system`: a success toast with an action stays until the next key
  press or click; one key chord format per platform.
- `view-protocol`: the Windows footer scenario names the chord "Ctrl K".
- `root-search`: the Windows hotkey scenario reads "Ctrl Alt V".
- `ai-commands`: a request that finishes while its view is held shows no
  toast over that view when the launcher reopens.

## Impact

- `src/ui/state.rs`: the success HUD in `task_changed`, the jump and the
  held-view rule in `Event::Activated`, and no expiry for a success toast with
  an action. `src/app/mod.rs`: `toggle` marks the open as the user's.
- `src/platform/mod.rs` (`ChordSeparator`) and `src/slint_shell/keys.rs`
  (`confirm_label`, `panel_label`): one format.
- `ui/shell/action-bar.slint`: the toast's key chord sizes to its text.
- Tests in each area. Stacked on `adopt-components`. Archive after it and after
  `task-registry`: the `root-search`, `ai-commands`, and `background-tasks`
  deltas build on `adopt-components`, and the `design-system` toast delta on
  `task-registry`.
