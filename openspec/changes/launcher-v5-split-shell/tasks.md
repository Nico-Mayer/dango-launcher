# Tasks

## 1. v5 part shapes (still one card)

- [ ] 1.1 Add v5 tokens to `ui/theme.slint` (pill height 52, shell gap 8, pill and card radii, shadow blur/offset/colour, shell margin) with matching constants in `src/slint_shell/render/height.rs`; `mise run check` passes
- [ ] 1.2 Restyle the back control as a chevron in a circle and the filter dropdown as a pill in `ui/shell/search-bar.slint`; UI test in `src/slint_shell/tests.rs` still finds both by accessible label, `cargo test` passes
- [ ] 1.3 Show the primary action and its key as one chip in `ui/shell/action-bar.slint`; existing ActionBar UI tests pass
- [ ] 1.4 Draw selected and hovered rows as rounded fills inset from the edges in `ui/views/list.slint` and `ui/shell/action-panel.slint`, keeping row heights; `cargo test` passes, including the list inset tests

## 2. Split shell

- [ ] 2.1 Split `ui/shell/launcher-window.slint` into a pill Rectangle and a card Rectangle (view, answer card, ActionBar, action panel) with the transparent gap; card always shown for now; UI test asserts the pill and card are separate elements with the gap between them
- [ ] 2.2 Update `render/height.rs` for pill 52 + gap 8 instead of SearchBar 64, and its unit tests for the new sums; `cargo test` passes
- [ ] 2.3 Add a full-window `TouchArea` under the pill and card that sends a new intent, handled in `Core` as an interrupting hide; recording-shell test in `src/app` asserts the hide is interrupting and state is held

## 3. Shadows and margin

- [ ] 3.1 Add drop shadows to pill and card from the `Theme` tokens and grow the window by the shell margin on every side; `height.rs` adds `2 * margin` and its tests cover it; `cargo test` passes
- [ ] 3.2 Windows: add `2 * margin` to the height used for centring in `platform/windows` `position_on_active_display`; `cargo clippy --all-targets -- -D warnings` passes
- [ ] 3.3 macOS: add the margin to the launcher frame in `platform/macos` placement; check symbol names against vendored crates, CI macOS build passes

## 4. Closed root view

- [ ] 4.1 Add the root-list flag to `State` in `src/ui/state.rs`: closed on reset and on a cleared query, held on interrupting hide; state machine tests in `src/ui/tests.rs` for open, typed, cleared, held, reset
- [ ] 4.2 Closed snapshot: no sections, or only "Needs attention" when an item is pinned, and a card-shown field; tests for bare pill, pinned card, and pinned provider order
- [ ] 4.3 Arrow Down on an empty prompt opens the list (first row from bare pill, next row from pinned card); Arrow Up does nothing; tests for each list-navigation scenario
- [ ] 4.4 Escape stage: clear query, then close open list, then dismiss; tests for each Dismissal scenario, including Escape on the pinned card
- [ ] 4.5 Renderer writes card-shown to `Shell`, the card hides, `height.rs` counts pill only or pill + gap + card; UI tests for bare pill (no list, no ActionBar), first keystroke, and height unit tests for each case
- [ ] 4.6 Update root-search UI tests that expect sections on open to open the list with Arrow Down first; `cargo test` passes

## 5. Toast capsule

- [ ] 5.1 Show the toast as a capsule under the pill while the card is hidden, from the same `Shell` toast properties; `height.rs` adds the capsule room; UI tests for capsule under bare pill and toast moving into the ActionBar after typing

## 6. Hand checks

- [ ] 6.1 Windows: with `dango-ui` and `mise run shot`, check bare pill, gap showing the app behind, shadows not cut, click in gap hides, SearchBar stays put while typing; note which renderer runs (shadows visible or not) in this task
- [ ] 6.2 macOS: check bare pill over another app, transparent area below the pill, whether a click there reaches the app behind or hits the launcher (both must hide it), shadows not cut, placement centred on the full frame
- [ ] 6.3 Add any platform trap found in 6.1 or 6.2 to `AGENTS.md`

## 7. Review

- [ ] 7.1 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context; report findings to the author

## Workflow follow-up

- Archive with `/opsx:archive` only after both platforms' hand checks are done.
