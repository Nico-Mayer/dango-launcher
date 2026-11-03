# Tasks

## 1. The calculator as a root provider

- [x] 1.1 Add `fend-core = "1.5.9"` to `Cargo.toml`. Add `src/extensions/calculator/` with `CalculatorExtension`:
  - a manifest named "Calculator" with `root_items: true`, listed in `builtin_manifests()`;
  - the gate from design.md (an operator or " to ", " in ", " as "; no comma between digits);
  - `fend_core::evaluate_preview_with_interrupt` with a 100 ms deadline, run on `spawn_blocking`;
  - a `RootProvider` that returns one `Source::Answer` candidate: id = the clean answer, title = the shown answer ("approx. " → "≈ "), actions "Copy answer" and "Paste answer";
  - `perform_action`: `copy` → `CopyToClipboard(item_id)`, and `paste` through the text exchange, as `extensions/ai` pastes.

  Add `Source::Answer`. Verify with unit tests in the module:
  - "2+2" → "4", "5 km to m" → "5000 m", "1.5 * 2" → "3", "17*23" → "391";
  - "5 km in miles" → title "≈ 3.1068559612 miles", id "3.1068559612 miles";
  - "firefox", "pi", "4k", "2fa", "1h", "1,5*2", "2048", and "" give none;
  - `copy` answers `CopyToClipboard`;
  - `paste` without a text exchange fails with "Dango can't reach other apps right now";
  - the manifest validates.
- [x] 1.2 Register the extension in `src/app/startup.rs` with the text exchange. Make the ranker in `src/ranking/` pass `Source::Answer` through first and unranked. In `run_action` (`src/app/mod.rs`), skip `record_use` for `dango.calculator`. Verify with:
  - a ranking test: an answer candidate comes first, above a pinned item;
  - an app test: an action on `dango.calculator` that answers `Done` records no frecency;
  - `cargo test`.

## 2. The answer card in root search

- [x] 2.1 In `src/ui/snapshot.rs`, split a `Source::Answer` item out of the root sections into `RootScreen.answer`, keeping it first in `root_order()`. Add `Theme.answer-height` (64 px), an `AnswerData` global (expression, answer, selected), `ui/shell/inline-answer.slint`, and `src/slint_shell/render/answer.rs` as the global's only writer.

  The card shows:
  - the query in muted text on the left, the `arrow-right` icon, and the answer large on the right;
  - the `selected` and `hover` fills as rows have;
  - a single click calls `Intents.activate`;
  - the accessible role `list-item` and the label "<expression> = <answer>".

  Place it under the SearchBar at root, start the list below it, and add its height in `render/height.rs`. Add the `calculator` and `arrow-right` icons to `src/slint_shell/icons.rs`. Verify:
  - **Reducer tests** in `src/ui/tests.rs`:
    - a new query with an answer selects it;
    - Arrow Down goes to the first result, and Arrow Up comes back and stays;
    - Enter runs "Copy answer";
    - the panel lists both actions;
    - the footer reads "Calculator", "Copy answer";
    - with only the answer, there is no empty view.
  - **UI tests** in `src/slint_shell/tests.rs`:
    - "2+2 = 4" is found and marked selected;
    - the window grows by 64;
    - an answer alone shows the card and no "No results".

## 3. Checks and review

- [x] 3.1 Run `mise run check` and `cargo test`. Verify both pass.
- [ ] 3.2 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context. Report the findings to the author. Verify that the findings are listed in the session.

## Workflow follow-up

- Change 4 of the design-system v2 stack, stacked on `task-registry` with git-spice.
- Archive only after `task-registry`: the `design-system` delta builds on its wording.
