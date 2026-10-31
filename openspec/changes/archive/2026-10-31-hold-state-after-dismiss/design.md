## Context

See proposal.md for motivation. What shapes the approach:

- Reset is welded to hide. `hide`, `hide_after_launch`, and `hide_for_work`
  in `src-tauri/src/lib.rs` each emit `dango://reset` the moment the window
  goes away, and `hide` and `hide_after_launch` also call
  `abandon_running_command`. The renderer's `resetToRoot` in `src/App.svelte`
  clears query, results, stack, panel, and view owner, then re-runs the empty
  search so the root results are painted before the next show.
- `hide_after_launch` serves two callers with different meanings. The action
  paths call it when a command has finished (`ActionOutcome::Done`, a
  successful launch, a hotkey-run command), and the window's
  `WindowEvent::Focused(false)` handler calls it when another application
  takes focus. The first is the user being done; the second is the user being
  interrupted. The same function cannot serve both once the two diverge.
- Focus loss reaches Rust by two routes: the renderer's `onblur` calls the
  `dismiss` command, and the window's native focus event fires on both
  platforms. Whichever runs second finds the window already hidden and does
  nothing, because `hide` checks visibility first. The native route exists
  because clicking away from the macOS non-activating panel does not reliably
  blur the webview.
- Escape pops views and clears the prompt before it hides, so an Escape
  dismissal always happens at an empty root. There is nothing to hold.
- Abandoning is a generation counter on `Invoker`. A sink whose generation is
  superseded drops its output. Nothing is torn down; the running task simply
  finds no one listening.
- `dango://activate` carries the latency probe id as `number | null`. The
  renderer focuses and selects the root input on every activation, which is
  what makes typing replace a held query, and would also pull focus out of a
  held pushed view.
- The `Launcher` config struct has single-word fields and no `rename_all`.
  `Selection` next to it uses `kebab-case`. The schema and `docs/config.md`
  are generated from the struct's doc comments, and a test fails when the
  reference is stale.
- `tokio::spawn` from outside the runtime panics, and the release profile
  aborts on panic. `tauri::async_runtime::spawn` is the handle that works from
  the main thread.
- The frontend is a renderer. Logic about why the window hid and how long ago
  belongs in Rust, which owns the window lifecycle.

## Goals / Non-Goals

**Goals:**

- One place decides what a hide means, and the renderer only receives
  `dango://reset` when a reset is wanted.
- Activation after the period is as fast as today: the root results are
  already painted when the window appears.
- Zero cost on the activation path within the period beyond one search for
  the held prompt.

**Non-Goals:**

- Anything under Non-goals in the proposal.
- Making the renderer aware of time. It never schedules anything.

## Decisions

### Hides carry a reason

A `HideReason` enum with three values replaces the implicit meaning of the
three functions: `Done` (an action finished, Escape at root), `Interrupted`
(shortcut toggle, focus loss, click away), and `ForWork` (a command hiding to
paste or read a selection). The three existing functions stay as thin entry
points that name their reason, with one change: the `Focused(false)` handler
stops calling `hide_after_launch` and calls the interrupted path instead. The
action-done callers keep `hide_after_launch` and its `Done` meaning, including
not restoring the previous foreground window.

What each reason does:

```
  reason        window   previous focus   running command        reset
  -----------   ------   --------------   --------------------   -----------------
  Done          hide     restore (1)      abandon now            emit now
  Interrupted   hide     restore          abandon when expired   emit when expired
  ForWork       hide     keep             keep                   emit now
```

(1) `hide_after_launch` keeps focus with the launched application, as today.

`Interrupted` with a period of zero behaves exactly as `Done`, with no timer.

Alternatives considered:

- A boolean `hold` parameter. Rejected: three behaviours do not fit a bool,
  and `ForWork` already differs from both others in the abandon column.
- Keeping `hide_after_launch` as the focus-loss handler and reading the
  reason from whether a command was running. Rejected: the focus event also
  fires after an action hides the window, and distinguishing the two cases by
  side state is exactly the ambiguity that produced the current coupling.

### The reset is timer-driven, not decided at show

An interrupting hide records a `Hold` in `AppState`: a generation number and
the handle of a task spawned with `tauri::async_runtime::spawn`. The task
sleeps for the period, then takes the state lock, checks that the hold with
its generation is still current, abandons the running command, and emits
`dango://reset`. `show` takes the same lock first, clears the hold, and
aborts the task. Any hide of another reason also clears the hold, so a
`ForWork` hide while held cannot leave a stale timer.

A held launcher also changes one thing in the output pump: a view arriving
while the window is hidden normally shows the launcher, because that is how a
command that hid itself to read the selection comes back. While a hold is
active the pump leaves the window hidden, so a surviving stream fills the
held view in the background instead of popping the launcher open on every
chunk.

Alternatives considered:

- Deciding lazily at show time by comparing `hidden_at` with the period and
  emitting `dango://reset` before `dango://activate`. Simpler, no task.
  Rejected because of what it does to the first frame: `resetToRoot` re-runs
  the empty search, and its results stream in after the window is already
  visible, so every activation after the period would paint an empty list and
  fill it a moment later. That is the most common activation, and today it
  paints complete because the reset ran at hide time. The timer keeps that
  property.
- One timer that only abandons, with the UI reset decided lazily. Rejected
  for the same first-frame reason, and for splitting one decision across two
  mechanisms.
- A renderer-side timer. Rejected: hidden webviews throttle timers, the
  renderer is not meant to hold logic, and the abandon has to happen in Rust
  anyway.

### The period is read at hide time

`hide` reads `launcher.reset_after` from the shared config held in
`Arc<FileConfig>` at the moment of the hide. The reload handler is untouched,
and a changed value applies to the next hide, which is what the spec promises.
A hold already in flight keeps the period it started with.

Alternative: caching the value in `AppState` and updating it on reload.
Rejected: one more thing for the reload handler to keep in step, for a value
read once per hide.

### Activation tells the renderer whether state was held

`dango://activate` changes its payload from the probe id to
`{ probe: number | null, held: boolean }`. On `held`, the renderer re-runs the
search for the current prompt so the results are fresh, and leaves focus
alone when a pushed view, the action panel, or the protocol error owns it. On
a plain activation it focuses and selects the root input as today.

This event is renderer plumbing, not part of the versioned view protocol. No
`protocolVersion` change and no manifest change.

Alternatives considered:

- Re-running the current search on every activation. Rejected: the root
  results after a reset are already painted, and re-running the empty search
  on activation could reorder frecent rows under the user's eyes while adding
  work to the activation path.
- A separate `dango://restore` event. Rejected: the renderer would have to
  order it against `dango://activate` for the latency probe, and one event
  with a flag has no ordering question.

### Focus on a held activation

When a pushed view is on screen the renderer does not move focus on
activation. Both webviews keep `document.activeElement` across a window hide
and show, so the field that had the cursor gets it back when the window is
focused. The existing cursor-owner effect in `App.svelte` already refuses to
touch focus while a view, the panel, or the protocol error is up, so the
activate handler only has to follow the same guard.

Alternative: asking the pushed view to refocus its input through a prop or an
exported method. Rejected until the platform check in the tasks shows the
webview does not restore it; the guard alone covers the expected case.

### The config field

`Launcher` gains `reset_after: Option<u32>` and the same
`#[serde(rename_all = "kebab-case")]` as `Selection`, so the key is
`launcher.reset-after`. `u32` makes the schema reject negatives and fractions
without a custom validator. The default is applied in code with
`unwrap_or(60)` and stated in the description, matching how `autostart`
documents its default. `is_empty` gains the new field.

The doc comment, which becomes the schema description and the reference
entry:

> Seconds the launcher keeps what you were doing after it is hidden by the
> hotkey or by losing focus. Pressing the hotkey again within this time
> continues where you left off. Default 60. 0 resets to root search at once.

Checked against the interface-copy spec: sentence case, written to the
reader, names the thing and what it does, no jargon.

### Fixtures and tests

`tests/fixtures/main.ts` emits the new activate payload, and gains a helper
that re-activates with `held: true` without emitting a reset. A browser test
types a prompt, re-activates held, and asserts the prompt is intact and that
`search` was called again with it; a second case opens a pushed view with a
focused field, re-activates held, and asserts focus did not move to the root
input. The Rust side gets a unit test on the config field's parse and
round-trip, in the style of the autostart test, and the settings reference is
regenerated.

## Risks / Trade-offs

- [A held AI stream keeps consuming tokens for up to the period after the
  user walked away] → Bounded by `reset-after`, which the user controls, and
  a deliberate hide still cancels at once. The default of 60 seconds keeps
  the waste small.
- [The timer fires in the same instant the user activates] → Both sides take
  the `Held` lock before acting and the timer checks it is still the current
  hold, so exactly one of them wins and the renderer sees a consistent
  sequence.
- [The hidden webview does not process `dango://reset` until shown, so the
  first frame after the period is stale] → Today's design already emits
  reset into the hidden window at hide time and relies on it being processed
  before the next show, so no new assumption is made. The desktop check after
  the period confirms it on both platforms.
- [The webview does not restore `activeElement` on one platform, leaving a
  held pushed view without a cursor] → The platform verification tasks check
  this explicitly. If it fails, the fallback is a refocus hook on the pushed
  view, scoped to that platform.
- [The native focus event and the renderer's blur both reach the interrupted
  path] → Unchanged from today: the visibility check in `hide` makes the
  second one a no-op, and both now mean the same thing.
