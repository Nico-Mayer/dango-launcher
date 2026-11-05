# Tasks

## 1. Runtime spike

- [ ] 1.1 Add `rquickjs` 0.14 (`futures`, `parallel`, `loader`, `macro`) and `rquickjs-extra` 0.3 (`timers`, `console`, `url`) to `Cargo.toml`; verify `mise run check` passes on Windows
- [ ] 1.2 Add a minimal session runner in a new `src/js/` module: fresh `AsyncRuntime` per session, load an ES module from a string, call an exported async function on the shell's tokio runtime, return its JSON result; verify with a `cargo test` that awaits a `setTimeout` inside the export
- [ ] 1.3 Add the memory limit and the interrupt handler tied to a cancel flag; verify with `cargo test` that an endless loop stops once the flag is set and that over-allocation returns an out-of-memory error
- [ ] 1.4 Add `ts-rs` 12 as a dev-dependency, derive `TS` for `ViewTree` under `cfg_attr(test, ...)`, and check the output for `tag = "kind"` enums and defaulted fields; verify with a `cargo test` that prints nothing and asserts the `View` union has a `kind` discriminant (fall back to `specta` per design if not)

## 2. Manifest and loading

- [ ] 2.1 Add the `manifest.json` DTO in `src/js/` mapping to `Manifest` (camelCase, `rootItems.refreshMinutes`), rejecting id mismatch and `services`; verify with `cargo test` cases for valid, bad JSON, id mismatch, and services
- [ ] 2.2 Add the folder scan of `<config_dir>/extensions/*/` (needs both files, missing folder is fine) and register each as `JsExtension` with `Host::Js`; verify with `cargo test` on a temp dir fixture that commands appear in the registry and a folder without `main.js` lands in `LoadReport`
- [ ] 2.3 Show JS load problems in the tray as "Extension <id> didn't load, see dango.log" through `load_problems`; verify with a `cargo test` on `load_problems`
- [ ] 2.4 Add a fixture extension under `tests/fixtures/extensions/` whose top level writes a marker file; verify with `cargo test` that root search finds its command and the marker is absent (spec: searching runs no extension code)

## 3. Commands and host API

- [ ] 3.1 Implement `JsCommand`: start a session, call the export named by the command id, push views as `ViewTree`s from JSON, finish on return or reported failure; verify with `cargo test` for a loading-then-content view command, a no-view success with message, and a reported failure
- [ ] 3.2 Map a missing export to "This command can't run. Its extension has no code for it." and uncaught errors or rejections to "This command stopped because of an error, see dango.log" with the error, stack, and ids in the log; verify with `cargo test`
- [ ] 3.3 Map out-of-memory to "This command used too much memory, so Dango stopped it." and stop a session when its invocation is cancelled; verify with `cargo test` that a looping command ends after cancel and that a pending timer never pushes a view
- [ ] 3.4 Add `dango.exec` on `platform::contained_command` with the agent PATH; verify with a `cargo test` running a program that exists on the test machine and checking code and stdout
- [ ] 3.5 Add `dango.fetch`, `dango.fs`, `dango.open`, `dango.copy`, `dango.paste`, `dango.env`; verify each with `cargo test` (fetch against a local `TcpListener` server, fs in a temp dir, open/copy/paste through the existing test fakes)
- [ ] 3.6 Add the `extension_storage` migration and `dango.storage`; verify with `cargo test` that values survive a reopened database and that two extensions with the same key read their own values
- [ ] 3.7 Allow extra keys in `extensions.<id>` in the config schema and pass them as `dango.preferences`; verify with `cargo test` that `user = "nico"` loads without a problem and reaches the extension, and that a reload changes the value for the next session

## 4. Actions

- [ ] 4.1 Route `perform_action` for JS extensions to the live session's handler table and return `ActionOutcome::Started`; verify with `cargo test` that an action can push a view, replace the view, show a ticker, go back, and finish
- [ ] 4.2 Deliver form values to the submit handler; verify with `cargo test` that "Fix login" reaches the handler for field "title"
- [ ] 4.3 End a session when its last view leaves the stack or the invocation is superseded; verify with a UI test in `src/slint_shell/tests.rs` that Escape out of a JS view stops its timer

## 5. Cached root items

- [ ] 5.1 Implement the cached `RootProvider` and builds on load, enable, and reload; verify with `cargo test` that rows match a query and that ten keystrokes run no JS
- [ ] 5.2 Add the `refreshMinutes` interval (stops on deactivate) and keep last rows on a failed build; verify with `cargo test` using tokio's paused time
- [ ] 5.3 Run a root item's actions through a new session calling `onAction`; verify with `cargo test` that Enter on a root item runs its first action

## 6. Live search (view protocol 3)

- [ ] 6.1 Add `filtering` to `ListView` and `GridView`, bump `PROTOCOL_VERSION` to 3, and reject version 2; verify with `cargo test` in `src/ui/tests.rs` that a version 2 tree shows the error
- [ ] 6.2 In the `src/ui/` reducer, skip narrowing for `"command"` views and emit `SEARCH_ACTION` with the query on each change; drop trees for older queries; verify with `src/ui/tests.rs` cases for delivery, out-of-order answers, no re-narrowing, the 50-item limit in command order, and query kept on going back
- [ ] 6.3 Hand the query and an abort signal to the JS search handler; verify with `cargo test` that a superseded search's result is never pushed

## 7. Reload and types file

- [ ] 7.1 Watch `extensions/` with `notify`, group events per folder, and register, replace, or unregister; verify with `cargo test` on a temp dir that editing `main.js` changes the next run, deleting a folder unregisters commands and releases hotkeys, and a broken manifest is reported while others keep working
- [ ] 7.2 Add the `dango.d.ts` template and the drift test that writes `assets/dango.d.ts`; write it at startup only when the folder exists; verify with `cargo test` that the checked-in file matches and that no folder means no file
- [ ] 7.3 Update `AGENTS.md`: replace "sandboxed host" with the trusted JS host, add the `src/js/` module to Architecture, and add any build fact found in group 1; verify by reading the diff

## 8. Platform checks

- [ ] 8.1 Windows: in a release build started with `DANGO_CONFIG_DIR` set to a scratch dir, run a fixture command that calls `dango.exec("gh", ...)`; check through `dango-ui` and `mise run shot` that no console window appears and the launcher keeps focus
- [ ] 8.2 macOS: start the bundle the Finder way (`env -i ... open`, see `AGENTS.md`) and run a fixture command that execs a Homebrew program; check through `dango-ui` that its output shows

## 9. Review

- [ ] 9.1 Run `mise run check` and `cargo test`; both pass
- [ ] 9.2 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context; report findings to the author

## Workflow follow-up

- Archive with `/opsx:archive` once 8.2 has been done on the Mac.
