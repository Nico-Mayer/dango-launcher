# Tasks

## 1. The calculator extension

- [ ] 1.1 Add `fend-core = "1.5.9"` to `Cargo.toml`. Add `src/extensions/calculator/` with `CalculatorExtension`:
  - a manifest with no commands and no root items, named "Calculator";
  - `answer(query) -> Option<Answer { expression, result }>`, which applies the digit-or-operator gate and calls `fend_core::evaluate_preview_with_interrupt` with a deadline interrupt, with no exchange-rate handler;
  - `perform_action` with `copy` → `CopyToClipboard(result)` and `paste` through the text exchange, as `extensions/ai` pastes.

  Verify with unit tests in the module:
  - "2+2" → "4", "5 km to m" → "5000 m", "1.5 * 2" → "3";
  - "firefox", "pi", "", and "2048" give none;
  - `copy` answers `CopyToClipboard`;
  - `paste` without a text exchange fails with its sentence;
  - the manifest validates.

  Fix the spec scenario strings if fend writes them differently.
- [ ] 1.2 Register the extension in `src/app/startup.rs`, hand it the text exchange, and hold it as `Services.calculator`. Verify with `cargo test` and an app test that "Calculator" is in the extension contexts.

## 2. The answer in root search

- [ ] 2.1 Add `Event::Answer { query, answer: Option<Answer> }`. In `Core::search`, run `calculator.answer` on `spawn_blocking` and apply the event. In the reducer:
  - keep the answer when the query matches, keep the last one until the next arrives, and clear it on an empty query;
  - select `ANSWER_ID` when a new answer arrives for a new query;
  - Arrow Up from the first result selects the card, and Arrow Down from the card selects the first result;
  - Enter on the card runs "Copy answer", and the panel lists "Copy answer" and "Paste answer", both as requests to `dango.calculator` with the answer as the item;
  - the footer reads "Calculator", "Copy answer";
  - with an answer and no results, there is no empty view.

  Put the answer in `RootScreen`. Verify with reducer tests in `src/ui/tests.rs` for each scenario in the `inline-answers` delta and the new `list-navigation` scenario, a stale answer for an old query being dropped, and an app test that typing "2+2" yields an answer in the snapshot.

## 3. The card

- [ ] 3.1 Add `Theme.answer-height` (64 px), an `AnswerData` global, `ui/shell/inline-answer.slint` (expression muted left, arrow, answer large right, `selected` fill, click to select, accessible label "<expression> = <answer>"), and `src/slint_shell/render/answer.rs` as its only writer. Place the card under the SearchBar at root and start the list below it. Add the card's height in `render/height.rs`. Verify with UI tests in `src/slint_shell/tests.rs`:
  - "2+2 = 4" is found, and is selected when the snapshot selects it;
  - the window height grows by 64;
  - with no results, the card shows and the empty view does not.

## 4. Checks and review

- [ ] 4.1 Run `mise run check` and `cargo test`. Verify both pass.
- [ ] 4.2 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context. Report the findings to the author. Verify that the findings are listed in the session.

## Workflow follow-up

- Change 4 of the design-system v2 stack, stacked on `task-registry` with git-spice.
- Archive only after `task-registry`: the `design-system` delta builds on its wording.
