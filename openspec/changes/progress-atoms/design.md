# Design

## Context

Loading today, from command to screen:

```
View{loading: true} --> reducer --> snapshot.busy --+--> Shell.loading (SearchBar sweep)
                                                    +--> busy toast "Loading…" (still Dot)
```

The reducer in `src/ui/` decides everything, and the renderer only copies the
snapshot into Slint globals. Timers already follow one pattern. The reducer
returns an `Effect` asking to be woken after a delay, `src/app/` runs the
timer, and it feeds an `Event` back with an id. Stale ids are ignored. Toast
expiry works this way (`Effect` with `TOAST_LIFETIME`, then
`Event::ToastExpired { id }`).

`ui/atoms/dot.slint` takes a `ToastStyle` and colours by it. The ActionBar and
the HUD both use it. The SearchBar draws its sweep inline with
`animation-tick()`.

## Goals / Non-Goals

**Goals:**

- One `Progress` atom (bar, ring, spinner), one status dot, one skeleton, all
  taking colours from `Theme` status roles.
- Loading-mark timing lives in the reducer, so it is tested without a window
  or a clock.
- Extensions can declare progress and status accessories.

**Non-Goals:**

- Task registry, ProgressView, toast actions (task-registry change).
- Skeletons for grid or detail.
- Root-search loading marks.

## Decisions

### Atoms, and why not std-widgets

| Part | Design part | File |
|---|---|---|
| `Progress` (`kind`: bar, ring, spinner; `value`: float, below 0 = indeterminate) | "Progress atom · bar · ring · spinner" | `ui/atoms/progress.slint` (new) |
| `Dot` with `status`: working, blocked, done, failed, queued | status dots ("same herdr status tokens") | `ui/atoms/dot.slint` (reworked) |
| `Skeleton` (rows) | "Skeleton · cold load" | `ui/atoms/skeleton.slint` (new) |

Slint 1.18's std-widgets have `Spinner` and `ProgressIndicator`. Both read
`FluentPalette` colours (checked in the vendored
`i-slint-compiler-1.18.1/widgets/fluent/spinner.slint` and
`progressindicator.slint`) and expose no colour property. AGENTS.md already
rules out std-widgets controls for that reason. We draw our own:

- **Bar:** a `Rectangle` track in `border`. Determinate fills a share of its
  width in `working`. Indeterminate is today's sweep, moved here unchanged:
  the segment stays `accent`, so the SearchBar looks as it does now.
- **Ring:** 16 px, a `Path` arc in `working` over a `border` track.
  Determinate draws the arc to `value`. Indeterminate draws a fixed quarter
  arc rotated by `animation-tick()`.
- **Spinner:** an indeterminate ring at icon size (the busy toast uses it at
  the dot's place, 12 px).

Indeterminate motion uses `animation-tick()`, as the sweep does. It runs only
while the part is on screen, so an idle launcher redraws nothing.

### Dot takes a status, toast styles map to it

`Dot` gets `in property <Status> status` (`working`, `blocked`, `done`,
`failed`, `queued`), coloured by `working`, `blocked`, `done`, `danger`, and
`idle`. Its accessible label is the state's word (see Copy). `ToastStyle`
stays where toasts need it. The ActionBar shows `Progress { kind: spinner }`
for busy, and `Dot` with `done` or `failed` otherwise. The HUD maps success to
`done` and failure to `failed`. `dot.slint` no longer imports `ToastStyle`.

### Loading-mark timing in the reducer

The snapshot's `busy` stops meaning "the view is loading". It means "the
loading marks are shown". A small state machine sits beside the view stack:

```
             view loading                   Due(id)
  Hidden ------------------> Pending -------------------> Shown
    ^                          |                          |    |
    |      view not loading    |       Held(id) and not   |    | view not loading
    +--------------------------+       loading            |    | before Held
    +-----------------------------------------------------+    v
    |                                                       Lingering
    +------------------------- Held(id) ----------------------+
```

- Entering `Pending` emits `Effect::WakeLoadingMarks { id, after: delay }`.
  `src/app/` answers with `Event::LoadingMarksDue { id }` after
  `LOADING_MARK_DELAY` (150 ms).
- Entering `Shown` emits a second wake for `LOADING_MARK_HOLD` (400 ms),
  answered with `Event::LoadingMarksHeld { id }`.
- `Shown` and `Lingering` are `busy` in the snapshot. `Hidden` and `Pending`
  are not.
- Leaving the view, a reset, or a new invocation goes straight to `Hidden` and
  bumps the id, so late wakes are ignored. That matches the "abandoned"
  scenario.

The two durations are constants in `src/app/mod.rs`, next to
`TOAST_LIFETIME`. The reducer only sees the events, so `src/ui/tests.rs` drives
every path by sending `LoadingMarksDue` and `LoadingMarksHeld` by hand.

Alternatives rejected:

- **Timing in Slint** (delay the `if Shell.loading` with a timer). That puts an
  interaction rule in the renderer, against AGENTS.md, and can't be tested in
  the reducer.
- **Debouncing in `src/app/`** before events reach the reducer. That hides
  state from the reducer, which builds the busy toast and the empty-state
  choice from the same flag.

### Skeleton rows

A list view marked loading with no items is the one case that today renders a
blank body. The snapshot gives it a `skeleton` flag in place of the empty
state. The renderer sets `ListData.skeleton` and the list draws `Skeleton` in
its body. The skeleton is not gated by the 150 ms delay: it replaces a blank
body, not content, so it has nothing to blink against.

Rows: `Theme.row-height` high, with an icon square and a title bar in
`raised`, and no motion. The count is fixed so that the window takes its
maximum height: `(MAX - SEARCH_BAR - ACTION_BAR - 2 * LIST_PADDING) / ROW`,
rounded down, so 8 at today's sizes. `render/height.rs` counts the same
number, per the fixed-height rule. The skeleton is one element with
`accessible-role: none` children. The list reports busy through the existing
"Loading…" toast.

### Protocol: two accessory kinds, version stays 2

```rust
Accessory::Progress { value: Option<f32> }   // {"kind":"progress","value":0.42}
Accessory::Status { status: Status }         // {"kind":"status","status":"needsInput"}
enum Status { Working, NeedsInput, Done, Failed, Queued }
```

The view-tree check that already refuses bad shortcuts also refuses a
`value` outside 0 to 1, with the same "error in place of the view" path. The
Slint side adds `progress` and `status` to `AccessoryKind`, and `value` and
`status` fields to `AccessoryData`.

This extends the versioned view protocol. It stays at version 2 because the
change is additive: every tree valid today is still valid. No third-party
extensions exist yet, so the only readers are the built-ins. The
task-registry change adds a view kind and decides there whether that needs
version 3.

`Status` is a protocol name. "Needs input" is the user-facing word, and the
herdr role behind it is `blocked`. The protocol uses `needsInput` so an
extension author writes what they mean.

### Copy

New strings, all accessible labels, none drawn as visible text. Written with
the ux-writing skill and checked against `interface-copy`: sentence case,
plain words, and matching the existing toast description "Failed".

| Where | String |
|---|---|
| Status dot, working | "Working" |
| Status dot, needs input | "Needs input" |
| Status dot, done | "Done" |
| Status dot, failed | "Failed" |
| Status dot, queued | "Queued" |
| Indeterminate ring or spinner | "Working" |
| Determinate ring or bar | no label, `accessible-value` 0 to 100 |

The busy toast keeps "Loading…".

## Risks / Trade-offs

- **Continuous redraws while busy.** `animation-tick()` repaints every frame
  while a spinner or sweep is on screen. That already happens for the sweep
  today. The spinner adds no cost beyond it, because both show together.
- **The software renderer.** It draws `Path` arcs without the GPU. A 12 to
  16 px arc is cheap, and the UI tests run on the testing backend, so a
  rendering glitch would only show by eye.
- **The 150 ms delay hides real short loads.** A load that ends within 150 ms
  shows nothing. That is the point. Skeleton rows still cover a cold list.
- **Flicker moves to the skeleton.** A list that loads in one frame shows one
  frame of skeleton in place of one frame of blank body. That is no worse
  than today.
