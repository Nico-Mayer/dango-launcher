## Context

The backend already owns search, ranking, invocation, the hold timer, and the
view trees. The frontend owns how the user moves through them:

- `src/App.svelte`: root query, results, `pickedId` and its fallback, the view
  stack, `viewOwner`, `shownInvocation`, `abandonedInvocation`, `liveQuery`
  stale filtering, `failure`, `workingTitle`, `protocolError`, the root action
  panel, Escape stages, Ctrl/Cmd+K, action shortcuts, result grouping, and the
  reactions to `dango://activate`, `reset`, `failed`, `results`, and `render`.
- `src/lib/ProtocolView.svelte`: per-view query and launcher-side filtering,
  per-view selection, available actions, primary action, Enter dispatch.
- `src/lib/ActionPanel.svelte`: panel selection and an exit guard that stops
  the closing key or click from reaching the parent during the exit
  transition.
- `src/lib/FormFields.svelte`: field values, template inspection per
  keystroke, Enter versus Cmd/Ctrl+Enter.
- `src/lib/input.svelte.ts`: pointer-versus-keyboard tracking for hover
  painting, and the gesture check that filters Bits UI's own selection writes.

The interface is 9 Tauri commands and 5 events. 12 Playwright files check
behaviour through fixtures that mock the IPC.

## Goals / Non-Goals

**Goals:**

- One Rust state machine that a Svelte renderer today and a Slint renderer
  next can both drive, with no interaction rule in either renderer.
- That state machine tested with `cargo test`, without Tauri or a window.
- No visible change.

**Non-Goals:**

- Slint, visual changes, packaging. See the proposal.
- Moving pure presentation into Rust: hover painting, scroll-into-view,
  caret placement, exit transitions.

## Decisions

### 1. A pure reducer: `Launcher::apply(Input) -> Vec<Effect>`

New module `src-tauri/src/ui/`:

- `state.rs`: `Launcher` holds all interaction state. No Tauri, no tokio, no
  I/O.
- `input.rs`: `Input` is either a user `Intent` or a backend `Event` (results
  arrived, view rendered, action finished, invocation failed, activated,
  reset).
- `effect.rs`: `Effect` is work the shell must do: search, run an action,
  invoke a command, cancel the invocation, dismiss, copy text, inspect a
  template.
- `snapshot.rs`: `Snapshot` is everything the renderer draws, derived from
  `Launcher`.

`lib.rs` keeps one `Mutex<Launcher>`. Every command and backend event goes
through `apply`, the shell runs the effects, and emits the new snapshot. The
async results of effects come back as `Event`s, so the reducer never awaits.

Rejected:

- **State-chart crates** (`statig`, `rust-fsm`). They model finite states and
  transitions. Ours is mostly data (lists, stacks, selections) with a few
  modes, which a struct plus `match` covers with less ceremony. Neither crate
  helps with the derived snapshot.
- **Reducer inside the Tauri commands** (state behind `AppState`, effects run
  inline). Simpler today, but then the tests need Tauri, and change 2 has to
  untangle it again.
- **Keep the state in Svelte and port it to `.slint` later.** It would be
  written twice, and Slint's language is a poor home for this logic.

### 2. Intents are semantic; keys go through a binding table

The renderer sends intents, not DOM events:

- `QueryChanged(text)`, `FieldChanged(id, value)`
- `Key(Chord)` for a chord the snapshot declares
- `Activate(item_id)` for a click on a row or an action
- `TogglePanel` for the Actions button, `ClosePanel` for a press outside it
- `Blur`
- `Ready` once the renderer listens, and `Painted(probe)` for the activation
  timing log, which replace `warmup_done` and `report_paint`

The snapshot carries `bindings`: the chords the state machine wants right
now. That is Escape, Enter, Up, Down, Cmd/Ctrl+K, Cmd/Ctrl+Enter in a
template form, and the shortcuts of the available actions. On keydown the
renderer checks the table synchronously. If the chord is there, it prevents
the default and sends `Key`. Otherwise the key stays with the focused control.

This is needed because a browser must call `preventDefault` synchronously, and
`invoke` is async. It also keeps text editing chords (Ctrl+A, Ctrl+V, Ctrl+Z,
arrows left and right) with the input in both renderers, without the state
machine listing them.

A binding can be marked as not applying inside a multiline field. Plain Enter
in a form is one: it submits from a text field but makes a newline in a
template field. The state machine does not track which field has focus.

Rejected:

- **The renderer maps keys to intents** (`MoveDown`, `OpenPanel`). Every
  renderer would then re-implement which key does what, per mode.
- **Forward every key and ask Rust whether it was handled.** Not possible
  synchronously from a webview.

### 3. One snapshot event, full replacement

After every `apply` that changed anything, the backend emits `dango://state`
with the full `Snapshot`. This follows the full-tree rule in `AGENTS.md`: no
patches, no diffing. The snapshot holds:

- root query with a revision, results grouped into sections with headings,
  the selected id, the failure and status text, the development flag
- the top of the view stack, filtered items, the per-view query, form values
  and template inspections, the selected id, the protocol error flag
- panel open state, its actions, and its selected id
- `focus`: which surface owns the keyboard (root input, view input, detail,
  form, panel)
- `bindings`

Streaming commands emit up to about 30 trees a second. Each one is now a
snapshot event of the same size as the tree plus a small header, so the
cost does not change in kind.

### 4. Text fields echo by revision, not by value

The input owns its text while the user types. Each keystroke sends
`QueryChanged`, but a snapshot answering an older keystroke must not overwrite
newer text or move the caret. So the snapshot carries `query_revision`, which
increases only when the state machine changes the query itself: Escape
clearing it, reset, a view pushed or popped. The renderer writes the query
into the input only when the revision changes. Form fields work the same way
with a per-form revision.

Rejected: always writing the echoed value back. That races with fast typing
and makes the caret jump.

### 5. Held state is just "do not reset"

Today the hold timer is in Rust and the held content is in Svelte. With both
in Rust, an interrupting hide leaves `Launcher` alone, activation with a hold
refreshes the results for the current query (an effect), and expiry applies
`Event::Reset`. The failure that survives reset (see `resetToRoot`) becomes a
field the reset leaves alone.

### 6. What stays in the renderer

- Hover painting: the pointer-versus-keyboard tracking in `input.svelte.ts`.
  Hover never selects, so it never reaches the state machine. The gesture
  check goes away, because Bits UI no longer writes the selection.
- Scroll-into-view when the snapshot's selected id changes.
- Focus placement for the `focus` field, and caret placement in forms.
- The action panel's exit guard, which only exists because of the exit
  transition. It goes away with the transition in change 2.
- Copy goes to Rust (`clipboard-rs` is already a dependency), so
  `navigator.clipboard` goes away.

### 7. Bits UI primitives

The surfaces keep their primitives: root search and pushed lists on
`Command`, the action panel on `Popover` plus `Command`, form fields on the
existing `Field`, `TextField`, and `Checkbox`.

`Command` is patched: its own keyboard navigation and selection are bypassed.
`value` is bound one-way from the snapshot, and Up, Down, and Enter are taken
from the binding table before `Command` sees them. The reason is that
selection is now the state machine's, and two owners of selection is exactly
the bug class `inUserGesture` exists to paper over. `Command` still provides
the listbox semantics, `aria-activedescendant`, and item markup.

### 8. Contracts

- The view protocol trees, items, actions, and the manifest do not change.
  `protocolVersion` is now checked in Rust when a tree arrives. A tree with an
  unsupported version sets the error screen in the snapshot.
- The frontend-backend interface is replaced, without compatibility (no
  users). `Snapshot`, `Intent`, and `Chord` are exported with `ts-rs` like the
  current protocol types, until change 2 removes `ts-rs`.

### 9. Strings

No string is added or changed. Strings the frontend composes today move into
the snapshot unchanged: "Needs attention", "Suggestions", "Everything else",
`No results for "{query}"`, `Running {title}…`, "Loading…", "Nothing to
show", "This view needs a newer version of Dango.", "Press Escape to go back."

### 10. Test migration

Each browser test goes into one of three buckets:

- **Moves to `cargo test`** (the logic is in the state machine now):
  `held-state` (3), `navigation` selection, replacement, stale-results, and
  streaming tests, `ActionPanel` selection, confirmation, empty-actions,
  carried-failure, and removal tests, `forms-and-failures` (3), `refinement`
  selection-survives and reset-during-panel, `presentation` protocol-version,
  and the `BusyStatus` status-text rules.
- **Stays in Playwright, dies in change 2** (webview
  presentation): `theme`, `visual-accessibility`, `ResultRow`, `BusyStatus`
  sweep and reduced motion, `ActionPanel` trigger keyboard, closing presence,
  held repeats, and long-actions scrolling, `navigation` hover and scrolling,
  `development-indicator`, `presentation` fill, `fixtures`.
- **Stays, then moves to a Slint test in change 2**: hover never selects,
  clicking keeps the typed query, the first row of a later group scrolls into
  view, the dev marker on every surface.

The exact per-test mapping is a task, recorded in `tests/browser/README.md`,
so change 2 can delete or port tests from a list.

### 11. Browser fixtures run the real state machine

Most tests that stay drive the launcher with keys (arrows, Cmd/Ctrl+K,
Escape). Fixed snapshots cannot answer them. So the fixture runs the real
`Launcher` in a small dev-only server, a cargo example
(`src-tauri/examples/ui_fixture.rs`) on `tiny_http`. It keeps one `Launcher`
per page session. The fixture's mocked `dispatch` posts intents to it, emits
the returned snapshot as `dango://state`, and answers the returned effects
(search, template inspection) from fixture data. Effects are recorded under
the old command names, so tests keep checking `calls`. Playwright starts the
server next to Vite.

This also lets the moved tests run once against the new interface before
they are deleted, which is the safety net named under Risks.

Rejected:

- **WASM build of `ui`.** `ui` would first move into its own crate, and the
  wasm toolchain would be removed again in change 2.
- **Scripted snapshots per test.** No reducer in the browser, so about 33
  tests would be rewritten and would no longer check keyboard behaviour end to
  end.
- **A TypeScript stand-in reducer.** The logic would exist twice.

## Risks / Trade-offs

- **Keystroke round trip.** Every key now waits for IPC before the selection
  moves. Tauri `invoke` on a warm webview is fast, and the search already
  takes this path. Typing itself does not wait, because the input owns its
  text (decision 4). If arrowing feels slower in a release build, the binding
  table could move the selection optimistically, but only after it is
  observed.
- **Scroll-into-view without `Command` driving it.** Bits UI scrolls on its
  own navigation. Setting `value` from outside may not scroll. The renderer
  then scrolls the selected row itself, and the group-heading margin trick
  (`scroll-mt-7`) has to keep working.
- **Hidden coupling in Svelte.** Some behaviour may live in markup or effects
  that are not obvious, such as focus hand-back after the panel closes.
  The browser tests are the safety net. They run against the new interface
  before any test is deleted.
- **Bigger snapshot per keystroke.** A full snapshot per search result batch
  instead of only the results. It stays bounded, because root search already
  sends only the top slice.
