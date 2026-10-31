## 1. Config setting

- [x] 1.1 Add `reset_after: Option<u32>` to `Launcher` in `src-tauri/src/config/mod.rs` with the doc comment from design.md, add `#[serde(rename_all = "kebab-case")]` to the struct, include the field in `is_empty`, and verify a parse-and-round-trip unit test in the style of the autostart test passes for `{ "launcher": { "reset-after": 300 } }` and for an absent field
- [x] 1.2 Add a schema test asserting that `reset-after` of -5 and of 1.5 are rejected and 0 is accepted, and verify `cargo test` passes
- [x] 1.3 Regenerate `docs/config.md` with `env DANGO_UPDATE_DOCS=1 cargo test the_settings_reference_is_current` and verify the reference lists `reset-after` under Launcher and the staleness test passes without the env var

## 2. Backend hide reasons

- [x] 2.1 Introduce a `HideReason` enum (`Done`, `Interrupted`, `ForWork`) and a `Held` slot in `AppState` holding the hide instant and a task abort handle, and verify `cargo check` passes
- [x] 2.2 Route `hide` (hotkey toggle and the `dismiss` command) through `Interrupted`: hide, restore focus, read `reset_after` from the shared `FileConfig` with default 60, and when the period is zero abandon and emit `dango://reset` at once, and verify with `DANGO_MEASURE=1` logging that a toggle with `reset-after: 0` behaves as today
- [x] 2.3 For a non-zero period, spawn the expiry task with `tauri::async_runtime::spawn`: sleep, take the state lock, confirm it is still the current hold, abandon the running command, and emit `dango://reset`, and verify in `tauri dev` that typing, toggling, and waiting past the period yields an empty prompt on the next activation
- [x] 2.4 Make `show` clear the hold and abort its task before showing, and emit `dango://activate` with `{ probe, held }`, and verify in `tauri dev` that a toggle within the period brings the typed prompt back
- [x] 2.5 Change the `WindowEvent::Focused(false)` handler to call the `Interrupted` path instead of `hide_after_launch`, keep the visibility guard so a focus event after an action's hide is a no-op, and verify that clicking away and re-activating within the period holds state while a launched application still keeps focus
- [x] 2.6 Make `hide_after_launch` (`Done`) and `hide_for_work` (`ForWork`) clear any pending hold so no stale timer survives, and verify a paste command run from a held view still comes back with its own view and is not cancelled

## 3. Renderer

- [x] 3.1 Update the `dango://activate` listener in `src/App.svelte` for the new payload: report the probe as before, on `held` re-run the search for the current prompt, and only focus and select the root input when no pushed view, action panel, or protocol error owns the cursor, and verify `npm run check` passes
- [x] 3.2 Update `tests/fixtures/main.ts` to emit the new activate payload, add a helper that re-activates with `held: true` without a reset, and document it in `tests/fixtures/README.md`, and verify existing fixture scenes still open
- [x] 3.3 Add a browser test that types a prompt, re-activates held, and asserts the prompt is intact, the selection is kept, and `search` was called again with the prompt; and a second case that opens a pushed view with a focused field, re-activates held, and asserts the root input did not take focus, and verify `npm run test:browser` passes
- [x] 3.4 Run `npm run check` and the full `npm run test:browser` suite and verify both pass with no changed expectations in existing tests

## 4. Verification on Windows

- [x] 4.1 With the default period, type a prompt, toggle the hotkey twice within a minute, and verify the prompt, results, and selected row are intact and the results were refreshed
- [x] 4.2 Open a pushed view with a text field, click another application, press the hotkey within the period, and verify the view is intact and the cursor is in that field
- [x] 4.3 Start an AI command that streams, toggle the hotkey while it streams, toggle again within the period, and verify the view shows the text so far and keeps updating; repeat and wait past the period, and verify the next activation is at root and the log shows the command abandoned
- [x] 4.4 Toggle the hotkey, wait past the period, activate, and verify the first painted frame shows the root results with no empty flash, and that `DANGO_MEASURE=1` activation timings stay within 80 ms
- [x] 4.5 Run an action that launches an application, re-activate, and verify the launcher is at root; press Escape at an empty root while a command streams and verify the command is cancelled
- [x] 4.6 Set `reset-after` to 300, then to 0, saving each time while Dango runs, and verify the next interrupting hide follows the new value without a restart

## 5. Verification on macOS

- [x] 5.1 Toggle the hotkey, wait past the period, activate, and verify the first painted frame of the panel shows the root results with no empty flash
- [x] 5.2 Repeat 4.2 on macOS, including a click away from the non-activating panel, and verify the cursor returns to the field; if it does not, record the finding in design.md before adding a refocus hook
