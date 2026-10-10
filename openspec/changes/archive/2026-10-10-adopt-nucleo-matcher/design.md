## Context

See proposal.md for motivation. What the swap has to fit:

- `DangoRanker` in `src-tauri/src/ranking/mod.rs` scores each candidate as
  the best of title, keyword and alias matches, with `NON_TITLE_PENALTY`
  taken off a non-title field, then adds `FRECENCY_WEIGHT * frecency` and
  `LEARNED_WEIGHT * prefix_weight`. All three constants are on the scale of
  the hand-rolled scorer: a matched character is worth 2, a run 7 more, a
  word start 9, the first character 10.
- The scorer returns the matched positions of a title match as character
  indices, and `ResultRow.svelte` highlights by those indices.
- Ranking runs per keystroke over up to a few thousand candidates with a
  30 millisecond budget asserted by a test.

## Goals / Non-Goals

**Goals:**

- One maintained dependency does the matching; Dango owns the field choice,
  the penalties and the weights, nothing else.
- Scores stay comparable across candidates the way they are today, so the
  ranker's shape and tests survive with constants retuned.

**Non-Goals:**

- Exposing fzf pattern syntax or multi-term matching.
- Any frontend change.

## Decisions

### Candidates evaluated

- `nucleo-matcher` 0.3.1, chosen. The engine behind Helix. fzf's scoring
  (16 per matched character, bonuses for word boundaries, camel case and
  consecutive runs, penalties for gaps), case folding and Latin accent
  normalisation of both sides, `fuzzy_indices` for highlight positions in
  character units, and a `prefer_prefix` option documented for the case where
  the user types the start of the match. The crate's own documentation calls
  the API finished.
- `fuzzy-matcher`, rejected. The skim engine behind a trait. Last release
  2020, no activity since, and its bonus system and case folding are ASCII
  only. The nucleo documentation names it as what nucleo replaces.
- `skim` as a library, rejected. It carries a whole terminal picker; the
  matcher inside is the one `fuzzy-matcher` wraps.

### A thin adapter, one per keystroke

`fuzzy.rs` keeps `Match { score, positions }` and replaces the dynamic program
with `Query`: built once per `rank` call from the typed text, holding the
crate's `Matcher` and one `Atom`, and scoring any number of texts. The
`Matcher` owns scratch buffers and is not `Sync`, which is why it is built
per call rather than kept in the ranker; construction is a few small
allocations, well under what one keystroke already spends.

The atom is `AtomKind::Fuzzy` with `CaseMatching::Ignore` and
`Normalization::Smart`, built with `Atom::new` rather than `Pattern::parse`,
so `^`, `$`, `'` and `!` in a query are ordinary characters.

Rejected: calling `Matcher::fuzzy_indices` directly with a pre-folded needle.
`Atom` already does the folding and normalisation of the needle and sets the
matcher config to match; doing it by hand would duplicate the crate.

### `prefer_prefix` on

The crate turns it off by default and recommends it "for autocompletion
usecases where the expectation is that the user is typing the entire match".
A launcher query is exactly that: the user types the start of a title. With
it on, `code` on "Code Runner" beats `code` on "Console Detach Element" by
more than the gap penalty alone gives, which is the ordering the existing
test asserts.

### Weights move to the new scale

The crate scores a matched character at 16 against the old 2, so the
constants beside the score are rescaled, not rethought:

```
 NON_TITLE_PENALTY   4  ->  8   half a boundary bonus
 FRECENCY_WEIGHT     6  -> 12   one recent launch still outweighs one character
 LEARNED_WEIGHT     10  -> 20   one learned `cal` still lifts Calendar on `c`
```

Rejected: normalising the crate's score back to the old range. A division
in the hot path to preserve numbers nobody depends on.

### Positions are character indices

`Utf32Str` indexes by `char`, as the old scorer did, so the frontend keeps
highlighting by the same unit. An accented title highlights correctly because
normalisation happens on the comparison, not on the text whose indices are
reported.

## Risks / Trade-offs

- [Scores of existing tests shift] → The ranker tests assert order, not
  numbers; they are the check that the rescaled constants keep the intended
  balance. Any that fail are retuned by changing a constant, not a test.
- [The crate's greedy fallback for long inputs] → It kicks in for needles or
  haystacks far longer than a title; launcher queries never reach it.
- [`Config` is `non_exhaustive`] → `Config::DEFAULT` is modified field by
  field, so a new field in a future release does not break the build.

## Migration Plan

No data changes. A user's frecency and learned inputs keep their meaning; the
weights that read them are rescaled with the score. Rollback is reverting the
commit.
