# Design

## Context

Busy state today, from reducer to screen:

```
state.working (root invocation) --+--> toast "Running X…"  --+
top view loading ----------------+--> toast "Loading…"     --+--> ActionBar (still Dot)
                                  +--> ViewScreen.busy -------> Shell.loading (SearchBar sweep)
```

`Snapshot::toast` (`src/ui/snapshot.rs`) picks "Running X…" while
`self.working` is set, and otherwise "Loading…" while the top view is loading.
The first render of an invoked command clears `working` (`state.rs`, in the
tree handler), so the toast passes from one source to the other. `RootScreen.busy`
mirrors `working`, but no renderer reads it.

Timers follow one pattern. The reducer returns an `Effect` with an id, and
`src/app/mod.rs` picks the duration, sleeps, and feeds an `Event` back. Stale
ids are ignored. Toast expiry works this way: `Effect::ExpireToast { id, style }`,
then `Event::ToastExpired { id }`.

`ui/atoms/dot.slint` takes a `ToastStyle`, and the ActionBar and the HUD both
use it with an empty label. The SearchBar draws its sweep inline with
`animation-tick()`, as element `SearchBar::sweep`.

## Goals / Non-Goals

**Goals:**

- One `Progress` atom (bar, ring), one status dot, and one skeleton, all
  colours from `Theme`.
- Busy-mark timing in the reducer, with one input, tested without a window or
  a clock.
- Extensions can declare progress and status accessories.

**Non-Goals:**

- Task registry, ProgressView, toast actions (task-registry change).
- Skeletons for grid or detail.
- Root-search result loading marks.

## Decisions

### Atoms, and why not std-widgets

| Part | Design part | File |
|---|---|---|
| `Progress` (`kind`: bar, ring; `value`: float, below 0 = indeterminate; `size` for rings) | "Progress atom · bar · ring · spinner" | `ui/atoms/progress.slint` (new) |
| `Dot` with `status`: working, needs-input, done, failed, queued | status dots ("same herdr status tokens") | `ui/atoms/dot.slint` (reworked) |
| `Skeleton` | "Skeleton · cold load" | `ui/atoms/skeleton.slint` (new) |

A spinner is not its own kind. It is a ring with no value at the size it is
needed: 12 px in the busy toast, 16 px as an accessory.

Slint 1.18's std-widgets have `Spinner` and `ProgressIndicator`. Both read
`FluentPalette` colours (checked in the vendored
`i-slint-compiler-1.18.1/widgets/fluent/spinner.slint` and
`progressindicator.slint`) and expose no colour property. AGENTS.md rules out
std-widgets controls for that reason, so we draw our own, following the
design's colours:

- **Track:** `raised` for both bar and ring.
- **Fill:** `working`.
- **Bar:** determinate fills a share of its width. Indeterminate sweeps a
  segment, using today's sweep maths.
- **Ring:** a `Path` arc. A full circle needs Slint's own workaround from
  `widgets/common/spinner-base.slint`: clamp the sweep to `min(0.9999, value)`,
  start the path with `MoveTo` and `LineTo` (femtovg does not fill a complete
  circle otherwise), and draw no arc at 0. Indeterminate is a fixed quarter arc
  turned with `transform-rotation` driven by `animation-tick()`
  (`transform-rotation` exists on every element in 1.18). The software
  renderer draws `Path`: its `path` feature is on through `std`.

Indeterminate motion runs only while the part is on screen, so an idle
launcher redraws nothing.

**The SearchBar's loading bar keeps today's look**, a 2 px line in `border`
with an `accent` segment, by choice. The design draws a 4 px bar in `working`
for progress inside views, but the SearchBar line is the launcher's quiet
loading signal and already matches the spec. It is a `Progress { kind: bar }`
with colour overrides (`track`, `fill` properties defaulting to `raised` and
`working`), so the SearchBar sets `border` and `accent`.

Accessibility: `accessible-role: progress-indicator`. Determinate sets
`accessible-value: round(value * 100)`, `accessible-value-minimum: 0`, and
`accessible-value-maximum: 100`. Rounding keeps 0.42 as 42, not
`42.000001`. Indeterminate sets the label "Working". The testing backend reads
`accessible_value()` (already used in `src/slint_shell/tests.rs`).

### Dot takes a status, and only accessory dots speak

`Dot` gets `in property <Status> status`, with the enum named after the
protocol (`working`, `needs-input`, `done`, `failed`, `queued`). It is coloured
by the `working`, `blocked`, `done`, `danger`, and `idle` roles. It has an
`in property <bool> announce`. Only the accessory sets it, and then the dot's
accessible label is the state's word. Toast and HUD dots keep no label, as
today. Their text and the existing "Failed" description already say it, so a
second "Failed" or "Done" would be noise.

The ActionBar shows a 12 px indeterminate `Progress { kind: ring }` for a busy
toast, and a `Dot` with `done` or `failed` otherwise. The HUD maps success to
`done` and failure to `failed`. `dot.slint` stops importing `ToastStyle`, which
moves to `ui/globals.slint` next to the toast properties.

### Busy-mark timing in the reducer

**One input:** `busy_input = working.is_some() || top view loading`. Both
toast texts and the SearchBar line follow the same marks, so passing from
"Running X…" to "Loading…" never drops the toast.

**State:**

```
             busy_input                Woke(id)
  Hidden ----------------> Pending{id} ----------> Shown{id, held: false}
    ^                         |                          |
    |  not busy_input         |                          | Woke(id)
    +-------------------------+                          v
    |                                         Shown{id, held: true}
    +--- not busy_input, held -------------------------+
```

- Marks show when `Shown && (busy_input || !held)`. When work ends before
  `held`, the marks stay. The second wake then finds `!busy_input` and goes to
  `Hidden`.
- Every entry into `Pending` takes a new id. A stale wake from an earlier
  Pending, or from an earlier view, never matches.
- A view replacement while `busy_input` stays true keeps the state. So does a
  streamed AI view: no reset, no new id, and the sweep's `animation-tick()`
  keeps running.
- A replacement that makes `busy_input` false: from `Pending` to `Hidden`;
  from `Shown{held: true}` to `Hidden`; from `Shown{held: false}` it stays until
  the wake.
- `busy_input` true again while `Shown{held: false}` lingers: stay `Shown`.
- Leaving the view, a reset, or the stack emptying: `Hidden` at once.

**Effect and event**, mirroring toast expiry:
`Effect::WakeBusyMarks { id, stage }`, with `stage` = `Delay` or `Hold`, and
`Event::BusyMarksWoke { id }`. The reducer knows from its state which stage a
wake is for. `src/app/mod.rs` holds `BUSY_MARK_DELAY` (150 ms) and
`BUSY_MARK_HOLD` (400 ms) and picks one by `stage`, next to `TOAST_LIFETIME`.

The snapshot reads the marks, not the view flag:

- `ViewScreen.busy` is true while marks show.
- `Snapshot::toast` returns the busy toast only while marks show, with the
  text chosen as today ("Running X…" while `working`, else "Loading…").
- `RootScreen.busy` is deleted. Its one test moves to the toast.

Alternatives rejected:

- **Timing in Slint** (a timer around `if Shell.loading`). That puts an
  interaction rule in the renderer, against AGENTS.md, and can't be tested in
  the reducer.
- **Debouncing in `src/app/`** before events reach the reducer. That hides
  state from the reducer, which builds the toast and the empty-state choice
  from the same flag.
- **Four states with `Lingering` and two events.** The `held` flag carries the
  same information with one state and one event fewer.
- **Delaying only "Loading…".** A root command would then show "Running X…" at
  once, drop the toast for 150 ms on its first render, and show "Loading…".
  That is a new blink on the most common path.

### Skeleton rows

The trigger is the tree, not the filter: a list view marked loading whose
declared items are empty. A refreshing list whose items are all filtered out
by the query shows its usual filtered state, not a skeleton.

- The snapshot gives the list body a `skeleton` flag in place of the empty
  state.
- `render/height.rs` sends a skeleton list to the existing `_ => MAX` arm.
  There is no row count in Rust.
- `Skeleton` fills its height with as many `Theme.row-height` rows as fit:
  `floor(height / row-height)`. Each row has an icon square and a title bar in
  `raised`, with no motion. The children use `accessible-role: none`, and the
  list reports busy through the toast.
- `ListData.skeleton` is written by the list renderer on every snapshot. The
  root path writes `ListData` too, so it sets `skeleton: false`.

The skeleton is not gated by the 150 ms delay. It replaces a blank body, not
content, so it has nothing to blink against.

### Protocol: two accessory kinds, version stays 2

```rust
Accessory::Progress { value: Option<f32> }   // {"kind":"progress","value":0.42}
Accessory::Status { status: Status }         // {"kind":"status","status":"needsInput"}
enum Status { Working, NeedsInput, Done, Failed, Queued }
```

An out-of-range value is clamped to 0 to 1 when rendered, with no refusal. A
float rounding error such as 1.0000001 must not replace a view with an error.
The user chose this.

This extends the versioned view protocol. It stays at version 2 because the
change is additive: every tree valid today is still valid. No third-party
extensions exist yet. The task-registry change adds a view kind and decides
there whether that needs version 3.

"Needs input" is the user-facing word, `needsInput` the wire name, and
`needs-input` the Slint enum value. The herdr role behind it is `blocked`.

### Copy

New strings, all accessible labels, none drawn as visible text. Written with
the ux-writing skill and checked against `interface-copy`: sentence case,
plain words, and matching the existing toast description "Failed".

| Where | String |
|---|---|
| Accessory status dot, working | "Working" |
| Accessory status dot, needs input | "Needs input" |
| Accessory status dot, done | "Done" |
| Accessory status dot, failed | "Failed" |
| Accessory status dot, queued | "Queued" |
| Indeterminate ring or spinner | "Working" |
| Determinate ring or bar | no label, value 0 to 100 |

"Running X…" and "Loading…" are unchanged. Only when they appear changes.

## Risks / Trade-offs

- **Continuous redraws while busy.** `animation-tick()` repaints every frame
  while a spinner or sweep is on screen. That already happens for the sweep.
  The spinner shows at the same time, so it adds no cost of its own.
- **The software renderer.** It draws small `Path` arcs cheaply. A glitch would
  only show by eye, because UI tests run on the testing backend.
- **A slow command's toast now waits 150 ms.** A command that finishes within
  that time shows no toast at all. This is the point of the rule.
- **Flicker moves to the skeleton.** A list that loads in one frame shows one
  frame of skeleton in place of one frame of blank body. That is no worse
  than today.
