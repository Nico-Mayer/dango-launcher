## Why

Dango will replace its webview with a native Slint UI (next change:
`replace-webview-with-slint`). Today the launcher's interaction state lives in
Svelte: `App.svelte`, `ProtocolView.svelte`, and `ActionPanel.svelte` own the
query, selection, view stack, action panel, Escape stages, shortcuts, and
failure messages. A new renderer would have to rewrite all of that. Moving it
into Rust first makes the swap a pure rendering job, and makes the behaviour
testable with `cargo test` instead of browser fixtures.

## What Changes

- Add a launcher state machine in Rust. It owns everything the user can change
  that is not pixels:
  - root query, results, and the selected row
  - the view stack, its owner, and stream replacement by invocation
  - stale-result and abandoned-invocation filtering
  - the action panel (open or closed, its selection) and action shortcuts
  - Escape stages, Ctrl/Cmd+K, Enter dispatch
  - list filtering inside pushed views, form field values
  - failure and "running" status, the protocol error screen
  - result grouping (needs attention, suggestions, everything else)
- **BREAKING (internal)** Replace the frontend-backend interface. The frontend
  sends user input as intents through one command. The backend answers with one
  event carrying a full UI snapshot. The current 9 commands and 5 events are
  replaced. The view protocol itself (trees, items, actions) is unchanged.
- The held state after an interrupting hide becomes "keep the Rust state", not
  "keep the Svelte state". The reset timer already lives in Rust.
- Copy-to-clipboard moves from `navigator.clipboard` to Rust.
- Svelte becomes a renderer of the snapshot. Hover painting, focus placement,
  scroll-into-view, and the action panel's exit guard stay in the renderer.
- Move the state behaviour now checked by Playwright to `cargo test`. Browser
  tests that check only presentation stay and are fed snapshots. Each
  remaining browser test is marked with whether it dies in change 2 or moves
  to a Slint test.
- Update `AGENTS.md` architecture notes with where the state machine lives and
  the intent/snapshot rule.

## Non-goals

- No Slint, no Tauri removal, no packaging changes.
- No visible change: same layout, strings, keys, and timing.
- No new features and no change to view protocol trees or the extension
  manifest.
- No compatibility with the old frontend-backend interface. There are no users.

## Platforms

Windows and macOS. The state machine is platform-neutral, and the only
platform difference it carries is Cmd versus Ctrl in key chords, which already
comes from Rust. Behaviour on both platforms stays as specified. Focus after
activation is renderer work that does not change, but it is re-checked by hand
on both platforms because the activation path changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `view-protocol`: the protocol version check and selection identity are
  Dango's job, not "the frontend's". Observable behaviour is unchanged.
- `development-indicator`: the renderer learns the build kind from the UI
  state, not from a request at mount. It still costs nothing on activation.

## Impact

- Rust: new module `src-tauri/src/ui/` (state, intents, snapshot, tests).
  `lib.rs` loses the per-command glue and gains one dispatch command and one
  snapshot event.
- Frontend: `App.svelte`, `ProtocolView.svelte`, `ActionPanel.svelte`,
  `FormFields.svelte`, `lib/types.ts`, generated `src/protocol/*` bindings.
- Tests: `tests/browser/*`, `tests/fixtures/*` (fixtures become snapshots),
  new Rust tests.
- Docs: `AGENTS.md`, `tests/fixtures/README.md`.
