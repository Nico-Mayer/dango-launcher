## Why

The fuzzy scorer in `src-tauri/src/ranking/fuzzy.rs` is hand-rolled. The
project rule is off-the-shelf first, with the design naming the candidates it
rejected; the M1 proposal even listed "a fuzzy matcher" among the crates it
would add, but none was added and no evaluation was recorded. A maintained
matcher exists that does this job better, so the scorer is debt to retire.

## What Changes

- The scorer becomes `nucleo-matcher`, the matching engine behind Helix: fzf
  scoring with word-boundary and camel-case bonuses, case folding and accent
  normalisation for non-ASCII text, and match indices for highlighting.
- The ranking layer keeps its own shape: a query is matched against title,
  keywords and alias, the best field wins, and frecency and learned inputs are
  added on top. The weights beside the score are retuned to the new scale.
- Typing an unaccented letter matches an accented title character, so `cafe`
  finds "Café". Today it does not.

Milestone: a follow-up to M1 core, which built the ranking pipeline. Platforms:
Windows and macOS, no platform-specific code.

## Non-goals

- Changing what matches: a subsequence still matches, an exact alias still
  wins, keywords and aliases still count.
- fzf pattern syntax (`^`, `$`, `'`, `!`). The query is one fuzzy term.
- Tuning frecency or learned inputs beyond moving their weights to the new
  score scale.
- Multi-term queries or word segmentation.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `root-search`: a new requirement states that matching folds case and accents
  and prefers word starts, which is the observable difference of the swap.
  "Ranking combines match quality and frecency" is untouched, since
  `add-adaptive-ranking` is modifying it.

## Impact

- `src-tauri/Cargo.toml`: adds `nucleo-matcher`.
- `src-tauri/src/ranking/fuzzy.rs`: the dynamic program is replaced by a thin
  adapter over the crate; the public shape of a match stays.
- `src-tauri/src/ranking/mod.rs`: `NON_TITLE_PENALTY`, `FRECENCY_WEIGHT` and
  `LEARNED_WEIGHT` move to the new scale.
- No change to the frontend, the extension manifest, the view protocol or the
  config file.
