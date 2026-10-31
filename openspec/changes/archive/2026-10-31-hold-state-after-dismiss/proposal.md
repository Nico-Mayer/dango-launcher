## Why

Every hide resets the launcher to root at once. Toggle the hotkey by reflex,
or let another window steal focus for a second, and a half-typed query, a
pushed view, or a streaming AI answer is gone. Raycast holds its state for a
while after the window closes, so pressing the shortcut again continues where
the user left off. Dango should do the same for hides that interrupt the user,
while keeping the instant reset for hides that mean the user is done.

## What Changes

- A hide that interrupts the user holds the launcher's state: hotkey toggle
  while visible, focus lost to another application, and a click away. The
  next activation within a grace period shows the launcher exactly as it was,
  with the current query re-run so the results are fresh. An activation after
  the period starts at root as today.
- A hide that completes the user's intent still resets at once: an action
  that finished and hid the launcher, and Escape on an empty root prompt.
- A command still running when an interrupting hide happens keeps running
  until the grace period ends. Reopening in time shows its output as if the
  launcher had never closed. Letting the period run out cancels it as a hide
  does today.
- A command that hides the launcher on its own behalf, to paste or to read a
  selection, is unchanged.
- The carried failure message keeps its existing behaviour.
- A new config setting, `launcher.reset-after`, sets the grace period in
  seconds. Default 60. Zero restores today's behaviour of resetting at once.
  It applies live on reload like every other launcher setting, and appears in
  the generated schema and settings reference.

Milestone: M7 polish. Platforms: Windows and macOS. The decision lives in
shared Rust and the renderer, so there is no platform-specific code. The two
platforms do differ in which signal reports focus loss, and both routes are
covered.

## Non-goals

- A "never reset" value. Zero and a large number cover the author's use.
- A pop-to-root shortcut like Raycast's Cmd+Esc. Escape already pops views and
  clears the prompt one step at a time.
- A per-action or per-command override of the reset behaviour, as Raycast's
  `popToRootType` offers extensions. The view protocol and action responses
  are not touched.
- Any visual cue that the launcher came back with held state.
- Persisting state across a restart of Dango.
- Changing what the first activation after startup shows.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `launcher-shell`: "Reset to root state on hide" becomes a reset that is
  immediate for deliberate hides and deferred by a configurable grace period
  for interrupting hides, including what happens to a running command. The
  "Dismissal" requirement gains the rule that an interrupting dismissal holds
  state.

## Impact

- `src-tauri/src/lib.rs`: the three hide paths gain a reason; an
  interrupting hide starts a cancellable timer that resets the renderer and
  abandons a running command when the period expires; showing cancels it. The
  period is read from the live config at hide time, so the reload handler
  needs no change.
- `src-tauri/src/config/mod.rs`: one new optional field on `Launcher`, with
  the doc comment that becomes its schema description.
- `docs/config.md` and the generated schema: regenerated from the struct.
- `src/App.svelte`: the activate handler stops forcing focus into the root
  input when a pushed view is on screen, and re-runs the current query when
  the launcher returns with held state.
- `tests/fixtures/main.ts` and `tests/browser/`: a fixture path for
  "activate without reset" and a test for focus and query behaviour on it.
- No new dependencies. No store or protocol changes.
