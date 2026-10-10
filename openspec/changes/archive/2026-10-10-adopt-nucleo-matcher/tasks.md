## 1. Dependency

- [x] 1.1 Add `nucleo-matcher` to `src-tauri/Cargo.toml` with default features; verify `cargo tree -i nucleo-matcher` from `src-tauri` shows it under `dango` and `cargo clippy -- -D warnings` passes

## 2. Scorer

- [x] 2.1 Replace the dynamic program in `src-tauri/src/ranking/fuzzy.rs` with `Query` over the crate's `Matcher` and a single fuzzy `Atom`, `prefer_prefix` on, keeping `Match { score, positions }`; verify the existing `fuzzy` tests pass unchanged plus new ones for an accented title matching an unaccented query with character positions, and for a query containing `^` matching literally
- [x] 2.2 Build one `Query` per `rank` call in `src-tauri/src/ranking/mod.rs` and pass it to `best_field_match`; rescale `NON_TITLE_PENALTY`, `FRECENCY_WEIGHT` and `LEARNED_WEIGHT` as design.md states; verify every `ranking` test passes, including the 2000-candidate budget test under 30 milliseconds

## 3. Verification

3.1 was run by the author on the installed Windows build and reported passing. 3.2 is carried forward unverified.

- [x] 3.1 Windows, release build with frecency reset: type `ze` and verify Zed ranks above Maximize; type `code` and verify a title starting with it ranks above one where the letters are scattered
- [ ] 3.2 macOS, release build: repeat 3.1; type `cafe` against an item titled with an accent, such as a snippet named "Café", and verify it matches and highlights the four characters

## 4. Wrap up

- [x] 4.1 Review the diff for comments that restate the code and for scope beyond the proposal; verify `cargo test`, `cargo clippy -- -D warnings` and `npm run check` pass
- [x] 4.2 Update `openspec/ROADMAP.md` under M1 with a note that the scorer is `nucleo-matcher` and what was verified per platform; verify the text matches the ticked tasks
