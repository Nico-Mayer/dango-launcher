# Tasks

## 1. v5 part shapes (still one card)

- [x] 1.1 Add pill height 52, shell gap 8 and pill/card radii to `ui/theme.slint`, with constants for pill height and gap in `src/slint_shell/render/height.rs`; `mise run check` passes
- [x] 1.2 Restyle the back control as a chevron in a circle and the filter dropdown as a pill in `ui/shell/search-bar.slint`; UI tests in `src/slint_shell/tests.rs` still find both by accessible label, `cargo test` passes
- [x] 1.3 Show the primary action and its key as one chip in `ui/shell/action-bar.slint`; existing ActionBar UI tests pass

## 2. Split shell

- [x] 2.1 Split `ui/shell/launcher-window.slint` into a pill Rectangle and a card Rectangle (view, answer card, ActionBar, action panel) with the transparent gap; card always shown, height from `Shell`; UI test asserts pill and card are separate elements with the gap between them
- [x] 2.2 `render/height.rs` counts pill 52 + gap 8 instead of SearchBar 64; the renderer writes the card height to `Shell` on both platforms and resizes the window only where `keeps_top_edge`; unit tests for the new sums and a UI test that a three-row card is shorter than the window when the window does not resize
- [x] 2.3 Add a `TouchArea` behind pill and card that sends a new intent, routed in `Core` into the `hide_on_focus_loss` path; recording-shell test in `src/app` asserts the click hides the launcher as an interrupting hide

## 3. Closed root view

- [ ] 3.1 Add the root-list flag to `State` and make `root_order()` yield only pinned items, or nothing, while the query is empty and the list closed; `reset()` and a cleared query close it. In the same commit, `Harness::root` and `root(count)` press Arrow Down after seeding. State machine tests: bare pill has no selection, Enter and Ctrl+K do nothing, a hidden item's shortcut does not fire, pinned items show alone in provider order, reset closes the list; `cargo test` passes
- [ ] 3.2 Arrow Down on an empty prompt opens the list (first row from bare pill, next row from the pinned card); Arrow Up does nothing; a typed character shows ranked results; state machine tests for each list-navigation scenario
- [ ] 3.3 Escape stage: clear query, then close the open list, then dismiss; tests for each Dismissal scenario, including Escape on the pinned card
- [ ] 3.4 Snapshot gains card-shown, the renderer writes it to `Shell`, the card hides, and `height.rs` counts pill only or pill + gap + card; UI tests for bare pill (no list, no ActionBar), first keystroke opening the card, and height unit tests for each case

## 4. Toast at the pill's end

- [ ] 4.1 While the card is hidden, draw the toast at the pill's right end from the same `Shell` toast properties, cut off before the query; UI tests for a toast on the bare pill (window height unchanged) and the toast moving into the ActionBar after typing

## 5. Hand checks

- [ ] 5.1 Windows: with `dango-ui` and `mise run shot`, check bare pill, gap showing the app behind, click in gap hides, pill-end toast, SearchBar stays put while typing; record in this task whether the machine runs femtovg or the software renderer
- [ ] 5.2 macOS: check bare pill over another app, card as tall as its content with a transparent area below, whether a click there reaches the app behind (both outcomes must hide the launcher), placement centred on the full frame
- [ ] 5.3 Add any platform trap found in 5.1 or 5.2 to `AGENTS.md`

## 6. Review

- [ ] 6.1 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context; report findings to the author

## 7. Design files

- [ ] 7.1 Update `Launcher Component System v5.dc.html` in `Downloads/Raycast-inspired launcher component system (2)/` to match the built app: sizes, split shell, bare pill, Arrow Down list, Escape stages, toast at the pill's end, no glass, no shadows (until the follow-up). Remove stale or false content, such as parts, themes, tokens or screens the app does not have. Verify by rendering it with headless Edge and comparing against `dango-ui` screenshots
- [ ] 7.2 After the author confirms 7.1, delete the older design files (`Launcher Component System.dc.html` and `v2` to `v4`) from that folder

## Workflow follow-up

- Archive with `/opsx:archive` only after both platforms' hand checks are done.
- Follow-up change: shadows with a window margin, if 5.1 shows femtovg.
- Follow-up change: rename "toast" to "status" and "HUD" to "notice" in specs, code and UI.
