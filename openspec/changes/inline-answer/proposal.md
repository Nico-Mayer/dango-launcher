# Proposal

## Why

Design system v2 adds an InlineAnswer: a card between the SearchBar and the
results that answers the query itself, so "2+2" or "5 km in miles" needs no
command. Dango has no calculator today. This is change 4 of the v2 stack.

## What Changes

- **New built-in extension `dango.calculator`:**
  - Answers math and unit conversions with `fend-core`, as root search's query
    changes.
  - Shows an answer only when the query holds a digit or an operator and the
    answer differs from the query, so app names never get a card.
  - Reads and writes a dot as the decimal separator on every machine.
  - Never blocks typing: an input that takes too long gets no answer.
- **InlineAnswer card** in root search, above the results: the expression on
  the left and the answer on the right.
  - It counts as the row above the first result. A new answer selects it, so
    Enter copies the answer and closes with the copied HUD.
  - Arrow Down moves into the results.
  - The action panel adds "Paste answer", which pastes into the app the user
    came from.
  - The card stays when no result matches, in place of the empty view.
- **Selection:** list-navigation counts the card as the first row of root
  search.

## Non-goals

- No currency: fend only converts it with an exchange-rate source, and none is
  wired.
- No time zones: `fend-core` 1.5.9 disables its clock.
- No error card for an input fend can't read; such a query simply gets no
  card.
- No calculator history, variables kept between queries, or settings.
- No answer card inside pushed views, only at root.

## Platforms

Windows and macOS. The calculator, the reducer, and the card are
platform-neutral. Paste reuses the existing text insertion that AI commands use
on both platforms, so it adds no platform code.

## Capabilities

### New Capabilities

- `inline-answers`: computed answers to the root query, covering what gets an
  answer, how the card shows, and what Enter and the panel do with it.

### Modified Capabilities

- `list-navigation`: in root search, an answer card counts as the row above
  the first result.
- `design-system`: root search can show an answer card between the SearchBar
  and the results.

## Impact

- `Cargo.toml`: `fend-core` 1.5.9 (MIT, no dependencies).
- New `src/extensions/calculator/`: the extension, its answer function, and
  its copy and paste actions. It is registered in `src/app/startup.rs`, with a
  handle in `Services`.
- `src/app/mod.rs`: `search` also asks the calculator, off the main thread, and
  sends `Event::Answer`.
- `src/ui/`: the answer in state and snapshot, selection, Enter, panel actions,
  and the footer.
- New `ui/shell/inline-answer.slint`, an `AnswerData` global, and
  `src/slint_shell/render/answer.rs`. Root height counts the card.
- Tests in each of those areas.
