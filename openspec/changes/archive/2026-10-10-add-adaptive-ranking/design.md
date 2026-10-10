## Context

See proposal.md for motivation. The pieces this builds on, as they are:

- `DangoRanker` in `src-tauri/src/ranking/mod.rs` scores each candidate as
  `fuzzy + 6.0 * frecency` and sorts by a two-value tier, `AliasExact` above
  `Match`. The empty query takes a separate path that orders by frecency and
  marks suggestions.
- `FrecencyTable` keeps a `HashMap` in memory, loads once at startup through
  `FrecencyPersistence`, and writes through on each launch. Ranking never
  touches SQLite.
- Launches are recorded in three places in `lib.rs`: `run_action` on a
  successful outcome, `run_command` for a command, and the application hotkey
  handler. None of them knows the query. The `search` command streams results
  per keystroke and holds no "current query" state.
- `src/App.svelte` has the query in scope at both root confirm paths,
  `confirm` for a command and `runAction` for a root item, and a separate
  `runViewAction` for actions inside a pushed view.
- The item id is already the stable key frecency uses, so it is the right key
  for learned pairs too.
- Migrations are numbered SQL files listed in `store/migrations.rs`; the last
  is `0005`.

## Goals / Non-Goals

**Goals:**

- A learned pair changes the result of a short query within one or two
  corrections, and stops mattering as soon as the user types past it.
- No new work on the search path beyond a lookup in memory.
- The rest of ranking is untouched, so existing tests keep passing unchanged.

**Non-Goals:**

- Any UI, including a way to see or clear learned pairs.
- Tuning frecency or the fuzzy scorer while in here.
- A general "context" model. One signal, the typed text.

## Decisions

### The typed text is a prefix of the stored input, not the other way round

Firefox's adaptive history fires a stored input when the text typed so far is
a prefix of it. This matches the complaint exactly: the user types `c`, gets
the wrong item, types `cal`, confirms Calendar. The pair stored is `cal`. Next
time `c` is typed, `cal` starts with `c`, Calendar is boosted. The longer
disambiguation teaches the shorter query.

It also gives the escape for free. Typing `calc` is not a prefix of `cal`, so
the learned pair falls silent and text takes over. More keystrokes mean more
textual control, fewer mean more learned control, with no mode or setting.

Rejected: firing when the stored input is a prefix of the typed text (`c`
learned once, then boosting on `calc` forever). That is the sticky behaviour
people complain about in launchers, and it cannot learn from a longer string.
Rejected: both directions with a distance weight. More knobs for no case the
first direction misses.

### Hybrid strength: exact input is a tier, prefix input is a bonus

```
 Tier::AliasExact   explicit config, untouched
 Tier::LearnedExact typed text == a stored input for this item
 Tier::Match        fuzzy + 6.0 * frecency + LEARNED_WEIGHT * prefix_weight
```

An exact repeat is the user having already answered what this text means, so
no fuzzy score should argue. Within the tier, items order by the pair's
decayed count, then title. A prefix hit is a hint: `c` learned from `cal` may
also compete with `c` learned from `chrome`, and both compete with a strong
word-start fuzzy match on a frequently used item. So it is a weighted bonus
inside `Match`, where frecency already lives, and the constant sits beside
`FRECENCY_WEIGHT` to be tuned by feel the same way.

Rejected: bonus only. On two-letter queries the bonus must be huge to beat a
word-start fuzzy match plus a high frecency, and a huge bonus is a tier with
extra steps. Rejected: tier for both exact and prefix. A single `cal` would
pin Calendar on `c` above a frequently used `Chrome` the user types `c` for,
which is the opposite complaint.

### Prefix weight and decay

A pair's weight is `count * 0.5^(age / HALF_LIFE)` with the frecency half life
of 30 days, so one constant governs forgetting in both tables. The prefix bonus
for a candidate is the sum of weights over every stored input that starts with
the typed text and belongs to that candidate. Summing rather than taking the
maximum rewards an item the user reaches through several longer strings.

Correction works without special cases: a second item confirmed for the same
text gets its own pair and the two compete on weight. The spec's "single slip"
and "repeated correction" scenarios fall out of count and decay alone.

### Where the pair is recorded

The frontend passes `query` on both root confirm paths; `run_action` and
`invoke_command` gain an optional `query: Option<String>` and record the pair
next to the existing `record_launch` when the outcome succeeds. `runViewAction`
passes nothing, and the hotkey handler calls neither, so neither teaches. The
text is lowercased and trimmed before storage; an empty result records
nothing, which also keeps the empty query clean.

Rejected: remembering the last query inside `SearchPipeline` so the backend
could record it without a frontend change. It would also record for view
actions and for a confirm that happens after the user cleared the field, and
it couples two things that are independent today.

### Hotkey launches leave ranking alone

Both hotkey dispatch paths credited frecency as if the user had searched for
the item. On the author's machine that put the window commands bound to
`hyper+enter` and `hyper+k` at 30 launches each in the last day, so `ze` ranked
Maximize above Zed by 130 points of frecency against a 25-point textual loss.
The user never typed those; the chord did. Frecency now means "reached through
the search box": `run_command` no longer records, `invoke_command` records
after it, and `dispatch_application` drops its record. The applications spec
is amended to match, since it promised the credit.

Rejected: counting a hotkey launch at a reduced weight. Any weight above zero
still accumulates from a chord pressed dozens of times a day, and nothing the
user types can push back. Rejected: saturating the frecency count instead.
Worth doing on its own merits, but it would leave the wrong signal in the table
and only dampen it.

### Storage and memory

Migration `0006_input_history.sql`:

```sql
CREATE TABLE input_history (
  id           TEXT PRIMARY KEY,
  input        TEXT NOT NULL,
  item_id      TEXT NOT NULL,
  use_count    INTEGER NOT NULL,
  last_used_at INTEGER NOT NULL,
  updated_at   INTEGER NOT NULL,
  deleted_at   INTEGER,
  UNIQUE (input, item_id)
);
```

Same columns and upsert shape as `frecency`, so the store code is a copy with
two keys. In memory, `InputHistory` holds a `BTreeMap<String, Vec<(item_id,
Record)>>` keyed by input. A keystroke does one range scan from the typed text
to the typed text with a high sentinel appended, which yields exactly the
stored inputs that start with it, then folds weights per item into a small
`HashMap` the ranker consults per candidate. For a two-letter query over a
history of a few thousand pairs this is a few hundred rows at most, well under
the 30 millisecond budget the existing test asserts.

Rejected: a flat `Vec` scanned per candidate. Quadratic in the wrong place.
Rejected: a trie. The `BTreeMap` range gives the same answer with no code to
own.

### Ids and orphans

Pairs are keyed by the same item id frecency uses. An application that is
uninstalled leaves orphan rows, as it does in `frecency` today; they match no
candidate and cost one map entry. No pruning in this change.

## Risks / Trade-offs

- [A learned exact tier can pin a wrong item for one text] → Only a repeat of
  exactly that text, and a repeated correction wins on weight. Typing one more
  character bypasses it entirely.
- [Two items learned for inputs that share a prefix fight on the short query]
  → Expected and correct; the heavier one wins and the user's next confirm
  settles it further. This is the behaviour being asked for.
- [Frontend and backend must agree on the query at confirm time] → The
  frontend passes the string from its own input binding at the moment of
  confirm; stale results after a fast edit confirm the item under the cursor
  with the current text, which is what the user sees.
- [Table grows without bound] → One row per distinct text and item. A heavy
  user produces low thousands over a year. Fine without pruning; decay keeps
  old rows weightless.

## Migration Plan

Additive migration, no data transform. Rollback is dropping the table; an
older Dango never reads it. Nothing changes for a user until they confirm
something after typing.
