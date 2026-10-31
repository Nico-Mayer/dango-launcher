## 1. Test inventory

- [x] 1.1 Write `tests/browser/README.md` listing every browser test with its bucket from design decision 10 (moves to `cargo test`, stays and dies in change 2, stays and moves to Slint). Verify every `test(` in `tests/browser` appears exactly once.

## 2. State machine, not yet wired

- [x] 2.1 Add `src-tauri/src/ui/` with `Launcher`, `Input`, `Intent`, `Event`, `Effect`, `Chord`, and `Snapshot` types, no Tauri or tokio imports. Verify `cargo test` and `cargo clippy --all-targets -- -D warnings` pass.
- [x] 2.2 Root search: query with revision, results, stale-query filtering, selection with first-row fallback, sections ("Needs attention", "Suggestions", "Everything else"), empty message. Verify with `cargo test ui::` tests ported from `navigation.spec.ts` (non-wrapping selection, replacement keeps identity or falls back, stale results dropped).
- [x] 2.3 View stack: push, same-invocation replacement, abandoned invocations ignored, owner tracking, protocol version check into the error screen. Verify with `cargo test ui::` tests for streaming replacement (30 replacements keep selection), late tree after Escape ignored, and unsupported version (from `presentation.spec.ts`).
- [x] 2.4 Pushed views: per-view query and launcher-side filtering, per-view selection, available actions (item actions win over view actions), primary action, Enter dispatch per view kind. Verify with `cargo test ui::` tests for list, detail, and form dispatch.
- [x] 2.5 Forms: field values seeded from the tree, form revision, Enter versus Cmd/Ctrl+Enter in template forms, submit once, template inspection effect. Verify with `cargo test ui::` tests ported from `forms-and-failures.spec.ts` and `presentation.spec.ts`.
- [x] 2.6 Action panel: open with Cmd/Ctrl+K only when actions exist, its own selection, confirm runs once and closes, Escape closes only the panel, all actions removed closes it, reset while open. Verify with `cargo test ui::` tests ported from `ActionPanel.integration.spec.ts` and `refinement.spec.ts`.
- [x] 2.7 Escape stages, action shortcuts, failure carried across reset and cleared by typing, acting, or Escape, "Running {title}…" status, and the `bindings` table per mode. Verify with `cargo test ui::` tests ported from `forms-and-failures.spec.ts` and `BusyStatus.spec.ts`, and a test that `bindings` holds the action shortcuts of the current selection.
- [x] 2.8 Hold: activation with a hold keeps everything and emits a search for the current query, reset clears all but the carried failure. Verify with `cargo test ui::` tests ported from `held-state.spec.ts`.

## 3. Switch the interface

This group lands as one commit so the app stays runnable.

- [x] 3.1 In `lib.rs`, replace the 9 commands and 5 events with one `dispatch(intent)` command and one `dango://state` event. Route search results, renders, failures, activation, reset, and action responses into `Launcher::apply`, and run the returned effects. Copy uses `clipboard-rs`. Verify `cargo test` and clippy pass.
- [x] 3.2 Export `Snapshot`, `Intent`, and `Chord` with `ts-rs`, and remove the bindings that are no longer used. Verify the CI binding diff check passes after `cargo test`.
- [x] 3.3 Rewrite `App.svelte` as a snapshot renderer: query echo by revision, keydown through `bindings`, clicks as `Activate`, focus from `snapshot.focus`, scroll-into-view on selection change. Verify `npm run check`.
- [x] 3.4 Rewrite `ProtocolView.svelte`, `ActionPanel.svelte`, and `FormFields.svelte` the same way. Keep the exit guard and hover painting, drop `inUserGesture`. Verify `npm run check`.
- [x] 3.5 Add the dev-only fixture server (`src-tauri/examples/ui_fixture.rs`, `tiny_http` as a dev dependency) from design decision 11, and start it from `playwright.config.ts`. Verify it answers an intent with a snapshot.
- [x] 3.6 Rewrite the fixtures to send intents to the fixture server and emit its snapshots instead of mocking the old commands, and update `tests/fixtures/README.md`. Verify `npm run test:browser` passes for every test still listed as staying, and run the tests listed as moved once against the new interface.

## 4. Remove what moved

- [x] 4.1 Delete the browser tests listed as moved to `cargo test`, after their Rust ports pass. Verify `npm run test:browser` and `cargo test` both pass, and the README list matches the remaining files.
- [x] 4.2 Update `AGENTS.md` architecture: the state machine lives in `src-tauri/src/ui/`, the renderer sends intents and draws snapshots, no interaction rule in a renderer. Verify with `hk check`.

## 5. Manual checks on the real OS

- [x] 5.1 Windows release build: first and held activation focus the prompt, arrowing scrolls, Ctrl+K and Escape stages behave, a held pushed view keeps its field focused, paste still reports a failure on next activation.
- [x] 5.2 macOS release build: the same checks with Cmd+K, and the NSPanel over a fullscreen app still takes typing.
