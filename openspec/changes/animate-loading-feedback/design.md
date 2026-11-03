# Design

## Context

- The loading bar is a 2px `Rectangle` in `ui/shell/search-bar.slint`, shown
  with `if Shell.loading`, filled with `Theme.accent`, carrying
  `accessible-role: progress-indicator` and the label "Loading…". The renderer
  sets `Shell.loading` from the snapshot (`src/slint_shell/render/shell.rs`).
- The busy toast reads "Loading…" for any loading view (`src/ui/snapshot.rs`).
- The AI command pushes a loading detail view with the body "Working…" before
  the request, then one loading detail view per chunk with the text so far,
  then the final view with `loading: false` (`src/extensions/ai/extension.rs`).
  `Request.provider` holds the config name of the provider.
- `design-system` forbade all motion when the Slint UI was built
  (`component-system`); no reason beyond scope was recorded.
- Slint 1.18 offers `animation-tick()`: milliseconds since start, and reading
  it asks for another frame. The testing backend has `mock_elapsed_time`.

## Goals / Non-Goals

**Goals:**

- The loading bar shows that work is going on, on every loading view.
- An AI command says which provider it is waiting for.

**Non-Goals:**

- See the proposal's non-goals. In short: no other motion, no spinner, no
  elapsed time, no reduce-motion handling, no protocol change.

## Decisions

### The sweep is a segment driven by `animation-tick()`

Inside the existing `if Shell.loading` bar, a clipped track in `Theme.border`
holds a child `Rectangle` in `Theme.accent`, about a quarter of the bar's
width. Its `x` is computed from `mod(animation-tick(), period) / period`,
moving it from fully left of the bar to fully right, then starting again.
Segment width and period live in `Theme` (`loading-sweep-width` as a fraction,
`loading-sweep-period` as a duration), next to the other sizes.

Because the binding only exists while the conditional element exists, Slint
asks for frames only while a view is loading. Replacing a loading view with
another loading view keeps `Shell.loading` true, so the element, and the
sweep's phase, which comes from the global tick, carry on without a restart.

The bar keeps its role and the label "Loading…" for assistive technology.

Alternatives rejected:

- std-widgets `ProgressIndicator { indeterminate: true; }`: does the same
  sweep, but hard-wires `FluentPalette.accent-background` and the rail colour,
  so it cannot take `Theme.accent` (the reason AGENTS.md gives for owning our
  controls). Its binding, `x: -parent.width + parent.width *
  mod(animation-tick(), 2s) / 1s` in a `clip: true` rectangle, is the model
  for ours.
- `animate x { iteration-count: -1; }` on a state change: it needs a property
  flip to start, and restarting it on every streamed replacement is easy to
  get wrong. The tick-based binding has no state.
- Ping-pong (left to right, then back): a one-way sweep is the common
  indeterminate pattern on both platforms.
- Pulsing the bar's opacity: a pulse on a 2px line is easy to miss, which is
  the problem being solved.
- A spinner icon in the toast's dot slot: a second moving element saying the
  same thing.

### The AI step is body text, not a protocol field

The first AI view's body becomes "Waiting for {provider}…", `{provider}` being
`Request.provider`. Chunk views and the final view are unchanged. Once text
arrives, the growing answer plus the sweep show that it is being written, so
no "Writing…" line is needed.

**View protocol:** unchanged. No field is added.

Alternatives rejected:

- An optional `loading_label` on list, grid, and detail views, shown in the
  busy toast ("Waiting for {provider}…", then "Writing…"): puts the text in
  the toast, but adds a field to a versioned contract, changes how the toast
  text is chosen, and touches every view literal, for a "Writing…" line the
  growing text already says. Cut after review.

### Strings

Checked with the ux-writing skill and against `interface-copy`. It follows the
existing busy texts ("Loading…", "Running {title}…"): a present participle,
front-loaded, ending in "…".

- New: "Waiting for {provider}…" (e.g. "Waiting for work…"), AI view body
- Removed: "Working…", AI view body

### Components

No new component. The sweep lives inside the SearchBar's existing loading bar
(`ui/shell/search-bar.slint`, design part: SearchBar). The toast stays the
ActionBar's toast with a still dot. The AI body is the existing detail view.

### Off the shelf

The one candidate, std-widgets' `ProgressIndicator`, is covered above. The
motion itself uses Slint's own `animation-tick()`.

## Risks / Trade-offs

- **Repaint cost on the software renderer.** On Windows without OpenGL 2,
  Slint falls back to the software renderer (AGENTS.md). The sweep asks for a
  frame every tick while loading. The winit software renderer can repaint only
  dirty regions when the surface reports buffer age; otherwise it repaints the
  whole window. Loading lasts seconds, so either is acceptable; the Windows
  hand check watches for lag while typing during a long AI request.
- **Motion was ruled out before** without a recorded reason. This design
  limits it to one 2px element shown only during work, so the original intent
  (a calm, instant interface) holds.
- **The toast and the body both speak while waiting** ("Loading…" and
  "Waiting for work…"). Accepted as the price of not changing the protocol.
- **A provider name may read oddly** (e.g. "Waiting for claude…" with a
  lower-case key). The user chose the name and can rename it; error messages
  already use it the same way.
