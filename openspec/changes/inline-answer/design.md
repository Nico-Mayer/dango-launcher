# Design

## Context

Root search runs on `Effect::Search(query)`. The pipeline asks every enabled
extension's `RootProvider` and cancels the query when the next keystroke comes.
It then ranks the candidates and streams them back as `Event::Results`. Root
selection is a result id (`picked`), with the first of `root_order()` as the
default, and `root_screen` already splits pinned items into their own section.
Actions run through the extension's `perform_action`.
`ActionOutcome::CopyToClipboard` copies, dismisses, and shows the "Copied to
clipboard" HUD. AI commands paste through the shared text exchange
(`text::TextTarget`). `run_action` records any `Done` as use, in frecency and
learned inputs. `Manifest::validate` rejects an extension that contributes
nothing.

## Goals / Non-Goals

**Goals:**

- The answer is one more root result, so selection, Enter, the panel, the
  footer, click, cancellation, and disabling need no new code paths.
- Answers never hold up typing or results.

**Non-Goals:**

- Currency, time zones, error cards, history (see the proposal).

## Decisions

### `fend-core` does the maths

The crates were checked during exploration (crates.io, October 2026), and
fend-core 1.5.9 was built and run in a scratch project during review:

| Crate | Fits | Why not |
|---|---|---|
| **fend-core 1.5.9** | Yes | - |
| cpc 5.0 | Partly | No preview mode, smaller unit set |
| numbat 1.24 | Partly | A whole language with its own REPL, heavy for one line |
| rink-core 0.9 | Partly | Unit-focused, needs data files |
| evalexpr, exmex, meval | No | Math only, no units. meval was last released in 2018 |

`evaluate_preview_with_interrupt(input, &context, &interrupt)` returns an empty
result for input it can't read, output that repeats the input, output longer
than 50 characters, and multi-line output. Verified outputs: "2+2" → "4",
"5 km to m" → "5000 m", "1.5 * 2" → "3", "17*23" → "391". "2048", "firefox",
"1password", "100 usd to eur", "today", and "2^1000" are empty. "9^9^9" is cut
off by a 100 ms deadline. Typical inputs take 10 to 300 µs. The default
decimal separator is the dot. fend-core has no dependencies and is MIT
licensed.

### The gate before fend

fend also reads things that aren't sums: "pi" → "approx. 3.14…", "4k" → "4000",
"2fa" → "2 fA", "1h" → "1 h", and "1,5*2" → "30" (the comma as a thousands
separator). So the query goes to fend only when:

- it contains an operator `+ - * / ^ %`, or " to ", " in ", or " as " between
  values;
- it has no comma between two digits.

"4k", "2fa", and "1h" have no operator and get no card. A German-style
"1,5*2" gets no card rather than a wrong one. The deadline (100 ms) is an
internal constant, not a spec number.

### The answer is a root result

`dango.calculator` (`src/extensions/calculator/`) is a built-in with
`root_items: true`. Its `RootProvider` returns, for a query that passes the
gate and gets an answer, one candidate:

- `source: Source::Answer` (new);
- id: the clean answer, with fend's "approx. " removed, which is what copy and
  paste use;
- title: the answer as shown, with "approx. " replaced by "≈ ";
- actions "Copy answer" (`copy`) and "Paste answer" (`paste`).

The card's expression is the query itself, so the candidate needs no new
field.

The work runs in the provider's async call on `spawn_blocking`, so a slow input
never stalls the pipeline. The pipeline already cancels it on the next
keystroke.

- **Ranking:** the ranker passes `Source::Answer` through unranked and first,
  like a pinned item but above it.
- **Snapshot:** `root_screen` splits it out into `RootScreen.answer`, as it
  does for pinned items. The renderer draws it as the card, not as a row.
- **Selection:** it stays in `root_order()` first, so the existing rules give
  everything:
  - a new query selects it;
  - Arrow Down goes to the first result, and Arrow Up from there comes back;
  - Enter, the panel, the footer context ("Calculator"), and click all work
    unchanged;
  - the answer can't go stale, because it is that query's own result.
- **Empty view:** the root empty view only shows when `results` is empty.
  With an answer there is a result, so the card stands alone.
- **Disabling:** turning the extension off removes its provider, so no card.
  The extension joins `builtin_manifests()`, which gives it a config schema
  entry and the manifest test.
- **Actions:** `perform_action(item_id, action_id)` reads the answer to copy
  or paste from `item_id`. The candidate's id carries the clean answer, with
  "approx. " removed. `copy` returns `CopyToClipboard(clean)`. `paste` inserts
  through the text exchange, as the AI paste does, and returns `Done`, or
  `Failed` with the text error's sentence.
- **Frecency:** `run_action` skips `record_use` for `dango.calculator`, so
  pasting "4" never becomes a learned input.

Rejected:

- **A separate `Event::Answer` path with a reserved selection id.** It needs
  special cases in `root_selected`, `step`, `confirm`, `run_available`,
  `actions_for`, `footer`, and `activate`, and stale-answer handling.
- **A manifest with no contributions.** `validate` rejects it, as
  extension-model requires.

### UI

| Surface | Design part | Where |
|---|---|---|
| Answer card | "InlineAnswer shell" | `ui/shell/inline-answer.slint` (new): expression in muted text on the left, an arrow icon, the answer in a larger face on the right; `selected` fill and `hover` fill as rows have; a single click activates, as a row's does |

No existing part fits: a list row is one line with a title and accessories,
and the card is a wider two-sided block outside the scrolled list. The design
draws unit labels apart from the values, but fend returns one string, so the
card shows it whole. Two more choices:

- **Height:** fixed, `Theme.answer-height` (64 px). `render/height.rs` adds it
  at root, and the list starts below it.
- **Data:** an `AnswerData` global (expression, answer, selected) with one
  writer, `src/slint_shell/render/answer.rs`.
- **Accessibility:** the role is `list-item`, and the label is
  "<expression> = <answer>".
- **Icons:** `calculator` (the footer context) and `arrow-right` are added to
  `src/slint_shell/icons.rs`.

### Copy

Written with the ux-writing skill and checked against `interface-copy`.

| Where | String |
|---|---|
| Card action, primary | "Copy answer" |
| Card action, panel | "Paste answer" |
| ActionBar context and extension name | "Calculator" |
| Approximate answer on the card | "≈ " before the answer |
| Card accessible label | "<expression> = <answer>", for example "2+2 = 4" |
| HUD after copying | "Copied to clipboard" (existing) |

### Contracts

There is no view-protocol or manifest-format change. `Source::Answer` is an
internal ranking source, not part of any contract.

### Stacking

The `design-system` delta copies "Every screen is one shell" and "Each view
kind has one layout" as `task-registry` leaves them. `task-registry` must be
archived first.

## Risks / Trade-offs

- **Enter copies where it used to launch** when a query gets an answer. The
  gate needs an operator or a conversion word, so a name such as "4k" or
  "1password" never gets a card. Arrow Down reaches the results.
- **Paste needs the previous app back in front.** It uses the same path as the
  AI paste, which is already verified on both platforms.
