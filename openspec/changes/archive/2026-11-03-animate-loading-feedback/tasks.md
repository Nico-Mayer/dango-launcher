# Tasks

## 1. AI command names its provider

- [x] 1.1 In `src/extensions/ai/extension.rs`, push the first view with the body "Waiting for {provider}…" from `Request.provider`, in place of `WORKING`, and remove the constant; verify by extending the existing streaming test: the first view is loading with markdown "Waiting for {provider}…" for the scripted provider's name, and the last view is not loading

## 2. Loading bar sweep

- [x] 2.1 Add `loading-sweep-width` (fraction of the bar) and `loading-sweep-period` (duration) to `ui/theme.slint`
- [x] 2.2 In `ui/shell/search-bar.slint`, turn the loading bar into a clipped track in `Theme.border` with an accent segment (with an element id) whose `x` comes from `animation-tick()` and the period, moving from fully left of the bar to fully right and starting again; keep the bar's role and "Loading…" label; replace the "Static" comment; verify `mise run check` compiles the `.slint` files
- [x] 2.3 Verify with one UI test in `src/slint_shell/tests.rs` on the Slint testing backend: with a loading view, find the segment by id, read its `absolute_position().x`, advance `mock_elapsed_time` by a quarter period, read again and assert it changed; then emit the view with loading false and assert the segment is gone
- [x] 2.4 Hand check on Windows with `dango-ui` (`mise run ui`): run an AI command against a slow provider; the body reads "Waiting for {provider}…" and the segment sweeps, the text grows as the answer arrives, and the sweep is gone when the answer is complete; typing in a pushed list while it loads shows no lag; if the machine uses the software renderer (no OpenGL 2), note it and check the same
  - Done 2026-11-03 by the author on Windows: looks fine.

## 3. Wrap up

- [x] 3.1 Run `mise run check` and `cargo test`; both pass
- [x] 3.2 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context; report findings to the author
