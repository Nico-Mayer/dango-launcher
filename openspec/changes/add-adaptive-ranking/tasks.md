## 1. Storage

- [ ] 1.1 Add `src-tauri/src/store/migrations/0006_input_history.sql` with the table from design.md and list it in `MIGRATIONS`; verify the migration tests pass with `cargo test migrations` from `src-tauri` and the recorded version is 6
- [ ] 1.2 Add `load_input_history` and `save_input_history` to `src-tauri/src/store/mod.rs` with the same upsert shape as frecency, keyed on `(input, item_id)`; verify with a store test that saves two pairs for one input, reloads them, and updates one in place

## 2. Ranking

- [ ] 2.1 Add `src-tauri/src/ranking/input_history.rs` with `InputHistory`: `load` through a persistence trait like `FrecencyPersistence`, `in_memory`, `record(input, item_id, now)` that normalises the text and skips an empty result, and `lookup(query, now)` returning per-item exact and prefix weights from a `BTreeMap` range scan with half-life decay; verify with unit tests for normalisation, the empty input, exact versus prefix, the `calc` release case, and decay
- [ ] 2.2 Add `Tier::LearnedExact` between `Match` and `AliasExact` in `DangoRanker`, order within it by learned weight then title, and add the prefix bonus with a `LEARNED_WEIGHT` constant beside `FRECENCY_WEIGHT`; verify with ranker tests for every scenario in the delta spec: exact wins over text, prefix boosts, release on a longer text, repeated correction wins, single slip does not, alias still first, empty query unchanged
- [ ] 2.3 Extend the existing 2000-candidate budget test with 2000 learned pairs and a two-letter query; verify it stays under 30 milliseconds

## 3. Recording

- [ ] 3.1 Give `run_action` and `invoke_command` in `src-tauri/src/lib.rs` an optional `query` and record the pair beside `record_launch` on success, leaving the hotkey handler alone; verify with `cargo clippy -- -D warnings` and by reading that only the two root paths record
- [ ] 3.2 Pass `query` from `confirm` and `runAction` in `src/App.svelte` and nothing from `runViewAction`; verify with `npm run check` and by confirming a result in a dev run and reading the new row in `input_history`

## 4. Verification

- [ ] 4.1 Windows, release build: type `c`, note the top item, type `cal`, confirm Calendar; clear and type `c` and verify Calendar is now first; type `calc` and verify the learned pair has no effect
- [ ] 4.2 Windows: confirm Calculator for `cal` three times and verify `cal` now ranks Calculator first; restart and verify it still does
- [ ] 4.3 Windows: open an application by its global hotkey and verify no `input_history` row is created; run an action inside a pushed view and verify the same
- [ ] 4.4 macOS, release build: repeat 4.1 and verify the same behaviour; verify the empty query suggestions are unchanged before and after
- [ ] 4.5 macOS: repeat 4.3

## 5. Wrap up

- [ ] 5.1 Review the diff for comments that restate the code and for scope beyond the proposal; verify `cargo test`, `cargo clippy -- -D warnings`, and `npm run check` pass
- [ ] 5.2 Update `openspec/ROADMAP.md` under M1 with a note on learned inputs and what was verified per platform; verify the text matches the ticked tasks
