# Proposal

## Why

Design system v2 adds an InlineAnswer: a card between the SearchBar and the
results that answers the query itself, so "2+2" or "5 km in miles" needs no
command. Dango has no calculator today. This is change 4 of the v2 stack.

## What Changes

- **New built-in extension `dango.calculator`:** a root items provider.
  - It answers math and unit conversions with `fend-core`, adding one answer
    result for the query.
  - It asks fend only when the query has an operator (+ - * / ^ %) or "to",
    "in", or "as" between values, and no comma between digits. So "4k",
    "2fa", "pi", and "1,5*2" get no card.
  - A dot is the decimal separator on every machine.
  - It never blocks typing: the pipeline cancels it per keystroke, and an input
    that takes too long gets no answer.
- **InlineAnswer card** in root search, above the results: the expression on
  the left and the answer on the right, with "≈" before an approximate answer.
  - It is the first row of root search, so a new query selects it. Enter
    copies the answer, without "≈", and closes with the copied HUD.
  - Arrow Down moves into the results.
  - The action panel adds "Paste answer".
  - The card stays when no result matches, in place of the empty view.
  - Copying or pasting an answer is not recorded in frecency.
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
platform-neutral. Paste reuses the text insertion AI commands use, which hands
focus back to the previous app on both platforms. That path is already
verified, and this change adds no platform code to it.

## Capabilities

### New Capabilities

- `inline-answers`: computed answers to the root query, covering what gets an
  answer, how the card shows, and what Enter and the panel do with it.

### Modified Capabilities

- `list-navigation`: in root search, an answer card counts as the row above
  the first result.
- `design-system`: root search can show an answer card. The card alone stands
  in for the empty view, and "No results" needs no answer.
- `interface-copy`: "No results for" shows only when the query gets no answer.
- `launcher-shell`: the window height counts the answer card.

## Impact

- `Cargo.toml`: `fend-core` 1.5.9 (MIT, no dependencies).
- New `src/extensions/calculator/`: the extension, its root provider, and its
  copy and paste actions. It is registered in `src/app/startup.rs` and listed
  in `builtin_manifests()`.
- `src/search/`: a new `Source::Answer`. `src/ranking/` puts it first.
- `src/app/mod.rs`: `run_action` records no use for the calculator.
- `src/ui/snapshot.rs`: `RootScreen.answer`.
- New `ui/shell/inline-answer.slint`, an `AnswerData` global, and
  `src/slint_shell/render/answer.rs`. Root height counts the card. Two icons
  are added to the registry.
- Tests in each of those areas.
