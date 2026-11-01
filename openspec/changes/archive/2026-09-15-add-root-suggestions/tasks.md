## 1. Mark suggestions in the ranker

- [x] 1.1 Add `suggested: bool` to `Candidate` in `src-tauri/src/search/mod.rs`,
      defaulting to false everywhere a candidate is built (every extension's
      provider and command source, plus the test helpers), and verify with
      `cargo check` and `cargo test` that nothing else changes.
- [x] 1.2 In `src-tauri/src/ranking/mod.rs`, add `SUGGESTION_LIMIT = 6` next to
      `FRECENCY_WEIGHT` and have `rank_empty_query` flag the leading items whose
      frecency score is above zero, up to that limit, after the existing sort.
      Verify with unit tests: a launched item is flagged and a never-launched
      one is not; with eight launched items exactly six are flagged and the
      other two follow unflagged ahead of never-launched items; with nothing
      launched nothing is flagged; a non-empty query flags nothing.
- [x] 1.3 Extend the existing 2000-candidate budget test with an empty-query
      run over a table where a dozen items were launched, and verify it stays
      under 30 milliseconds.
- [x] 1.4 Add `suggested` to `ResultItem` in `src-tauri/src/lib.rs`, copied in
      `ResultItem::new`, and verify with `cargo test` and by running the dev app
      that the `dango://results` payload for an empty query carries it (a
      `console.log` of the payload while developing, removed before commit).
      Done: the field is on the serialised struct with the other camelCase
      fields.

## 2. Render the sections

- [x] 2.1 Add `suggested: boolean` to `ResultItem` in `src/lib/types.ts` and
      verify with `npm run check`.

## 3. Verification

- [x] 3.4 Run `cargo clippy -- -D warnings`, `cargo test`, and `npm run check`,
      and verify all three pass.
