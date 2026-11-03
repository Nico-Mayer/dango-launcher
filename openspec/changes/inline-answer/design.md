# Design

## Context

Root search runs on `Effect::Search(query)`: `Core::search` streams the
pipeline's results back as `Event::Results { query, .. }`, and the reducer keeps
them only when `query` still matches. Root selection is a result id (`picked`),
with the first row as the default. Actions on a result go to their extension's
`perform_action`. `ActionOutcome::CopyToClipboard` already copies, dismisses,
and shows the "Copied to clipboard" HUD. AI commands paste through the shared
text exchange (`text::TextTarget`), which works on both platforms.

## Goals / Non-Goals

**Goals:**

- One small extension that answers a query, and a card that reuses the root
  selection and the action path.
- Answers never hold up typing or results.

**Non-Goals:**

- Currency, time zones, error cards, history (see the proposal).

## Decisions

### `fend-core` does the maths

The crates were checked during exploration (crates.io, October 2026), and
fend-core's source was read:

| Crate | Fits | Why not |
|---|---|---|
| **fend-core 1.5.9** | Yes | - |
| cpc 5.0 | Partly | No preview mode, smaller unit set |
| numbat 1.24 | Partly | A whole language with its own REPL, heavy for one line |
| rink-core 0.9 | Partly | Unit-focused, needs data files |
| evalexpr, exmex, meval | No | Math only, no units. meval was last released in 2018 |

fend-core has no dependencies. `evaluate_preview_with_interrupt(input,
&context, &interrupt)` is built for live preview. It returns an empty result
when the input fails, when the output repeats the input, when it is longer
than 50 characters, or when it spans several lines. Its `Interrupt` trait lets
a deadline stop a slow input. Currency needs an exchange-rate handler; without
one, currency units fail, which is the wanted non-goal. The decimal separator
stays fend's default, the dot.

**The gate before fend:** the query must contain an ASCII digit or one of
`+ - * / ^ % ( )`. That keeps "pi", "c", and "kg" from turning into cards. The
deadline is an internal constant (100 ms) in the extension, not a spec number.

### The extension: `dango.calculator`

- New module `src/extensions/calculator/`, with a manifest that has no
  commands and no root items, registered in `startup.rs`.
- `CalculatorExtension::answer(&self, query) -> Option<Answer { expression,
  result }>`.
- `perform_action(item_id, action_id)` takes the answer as `item_id`, so the
  extension keeps no state:
  - `copy` returns `CopyToClipboard(result)`;
  - `paste` inserts through the text exchange, as the AI paste does, and
    returns `Done`, or `Failed` with the text error's sentence.
- `Services.calculator: Option<Arc<CalculatorExtension>>`, the way
  `applications` is held.

### The answer reaches the reducer like results do

`Core::search(query)` also runs `calculator.answer(query)` on
`spawn_blocking` and applies `Event::Answer { query, answer }`.

- The reducer keeps the answer only when `query` still matches.
- It keeps the last answer until the next one for the current query arrives,
  so the card doesn't blink while the user types.
- An empty query clears it.

Rejected: answering in the reducer (fend could stall the main thread), and an
answer as a ranked root candidate (the design keeps it outside the list,
never filtered, always first, and alive when no result matches).

### Selection: the card is row 0

The card uses a reserved selection id, `ANSWER_ID`, in the existing root
selection.

- A new answer for a new query selects it, the same way a new query selects
  the first row today.
- Arrow Up on the first result selects it; Arrow Down from it selects the
  first result.
- Enter and the panel read the selected id: the card's actions are "Copy
  answer" (primary) and "Paste answer", run as `ActionRequest`s to
  `dango.calculator` with the answer as `item_id`.
- The footer's context is "Calculator" with the extension's icon.
- With an answer and no results, the empty view is not shown.

### UI

| Surface | Design part | Where |
|---|---|---|
| Answer card | "InlineAnswer shell" | `ui/shell/inline-answer.slint` (new): expression (muted, left), an arrow, answer (large, right); `selected` fill when selected; click selects, double click confirms |

No existing part fits: a list row is one line with a title and accessories,
and the card is a wider two-sided block that is never filtered or scrolled
with the list. The card has a fixed height, `Theme.answer-height` (64 px).
`render/height.rs` adds it at root, and the list starts below it. An
`AnswerData` global has one writer, `src/slint_shell/render/answer.rs`. The
card's accessible role is `list-item` with the label "<expression> = <answer>".

### Copy

Written with the ux-writing skill and checked against `interface-copy`.

| Where | String |
|---|---|
| Card action, primary | "Copy answer" |
| Card action, panel | "Paste answer" |
| ActionBar context and extension name | "Calculator" |
| Card accessible label | "<expression> = <answer>", for example "2+2 = 4" |
| HUD after copying | "Copied to clipboard" (existing) |

### Contracts

There is no view-protocol or manifest change. The answer is a launcher feature,
not a protocol view, and the new extension's manifest uses existing fields
only.

### Stacking

The `design-system` delta copies "Every screen is one shell" as
`task-registry` leaves it, with the progress view. `task-registry` must be
archived first, or this delta would drop that view when synced.

## Risks / Trade-offs

- **Enter copies where it used to launch.** When a query gets an answer, the
  card is selected, so Enter copies instead of running the first result.
  Arrow Down reaches the results. Queries that look like app names rarely
  qualify: "2048" alone echoes itself, so fend's preview drops it, and
  "1password" holds a digit but isn't something fend can read.
- **fend's output format:** results such as "approx. 3.1415926535" are shown
  and copied as fend writes them.
