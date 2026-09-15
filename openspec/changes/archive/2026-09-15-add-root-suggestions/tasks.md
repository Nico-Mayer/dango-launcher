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
      fields; the live payload check is covered by the headings appearing in
      3.1 and 3.2, since they render only when the flag arrives.

## 2. Render the sections

- [x] 2.1 Add `suggested: boolean` to `ResultItem` in `src/lib/types.ts` and
      verify with `npm run check`.
- [ ] 2.2 In `src/App.svelte`, derive the marked prefix and the rest from
      `results`, and when the prefix is non-empty render two `Command.Group`
      blocks with `Command.GroupHeading` texts "Suggestions" and "Everything
      else" and `Command.GroupItems` around the existing `Command.Item` and
      `ResultRow` markup. Keep the flat `{#each}` for the unmarked case. Verify
      with `npm run check` and by opening the dev app: with items launched the
      two headings show; with a fresh frecency store, or after typing one
      character, no heading shows.
      Code done and type-checked. The on-screen check is still to run; it is
      the same check as 3.1 and 3.2.
- [ ] 2.3 Style the heading with the muted small-text treatment from design.md,
      aligned with the row padding, and verify visually in both the light and
      the dark theme that it reads as a label and not as a row.
      Styled as in design.md; the visual check on both themes is still to run.
- [ ] 2.4 Verify list-navigation still holds in the grouped list: the first
      suggestion is selected on open, Arrow Down crosses from the last
      suggestion into "Everything else" without a stop, Arrow Up on the first
      row keeps it selected with the "Suggestions" heading fully visible, click
      confirms the clicked row, and hover on a row inside a group is the weaker
      highlight only.
      Not yet run. Note for the tester: the primitive's own heading scroll never
      fires (see design.md), so the first-row case rests on the existing
      scroll-to-top effect and the second heading on a scroll margin.

## 3. Verification

Archived 2026-09-15 on the author's confirmation that the result looks fine
after using it. The checks below were not ticked one by one, so treat any that
matters as unexercised rather than passed.

- [ ] 3.1 Verify on macOS: a fresh frecency store shows a flat unlabelled
      list; launching three items and reopening shows them under
      "Suggestions" with the rest under "Everything else"; launching a seventh
      distinct item shows six suggestions and the least frecent launched item
      first under "Everything else"; no item appears twice; typing removes the
      headings and deleting back to empty restores them.
- [ ] 3.2 Verify on Windows: the same list as 3.1, with attention to the
      grouped markup in WebView2, the hover highlight inside a group, and the
      scroll position when arrowing back to the top.
- [ ] 3.3 Verify on both platforms with `env DANGO_MEASURE=1` that hotkey to
      painted window stays under 80 milliseconds with the grouped list showing.
- [x] 3.4 Run `cargo clippy -- -D warnings`, `cargo test`, and `npm run check`,
      and verify all three pass.
