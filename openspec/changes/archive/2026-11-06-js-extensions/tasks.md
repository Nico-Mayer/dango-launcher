# Tasks

## 1. Runtime spike (Windows)

- [x] 1.1 Add `rquickjs` 0.14 (`futures`, `parallel`) and `rquickjs-extra` 0.3 (`timers`, `url`) to `Cargo.toml`; verify `mise run check` passes on Windows
- [x] 1.2 Add a minimal session runner in a new `src/js/` module: fresh `AsyncRuntime` per session, load an ES module from a string, call an exported async function on the shell's tokio runtime, return its JSON result; verify with a `cargo test` that awaits a `setTimeout` inside the export
- [x] 1.3 Add the memory limit and the interrupt handler tied to a cancel flag; verify with `cargo test` that an endless loop stops once the flag is set and that over-allocation returns an out-of-memory error
- [x] 1.4 Add `ts-rs` 12 as a dev-dependency, derive `TS` for `ViewTree` under `cfg_attr(test, ...)`, and check `tag = "kind"` enums and `#[serde(default)]` fields; verify with a `cargo test` asserting the `View` union has a `kind` discriminant and defaulted fields are optional (fall back to `specta` per design if not)

## 2. Manifest and loading

- [x] 2.1 Add the `manifest.json` DTO in `src/js/` mapping to `Manifest` (camelCase, `rootItems.refreshMinutes`, relative `icon` resolved against the folder), rejecting id mismatch and `services`; verify with `cargo test` cases for valid, bad JSON, id mismatch, services, and a relative icon
- [x] 2.2 Add the folder scan of `<config_dir>/extensions/*/` (missing folder is fine, a folder with neither file is ignored, one file only is reported) and register each `JsExtension`; verify with `cargo test` on a temp dir that commands appear in the registry, a folder without `main.js` lands in `LoadReport`, and a `.git` folder is not reported
- [x] 2.3 Show JS load problems in the tray as "Extension <id> didn't load, see dango.log" through `load_problems`; verify with a `cargo test` on `load_problems`
- [x] 2.4 Add a test extension (in a temp dir) whose top level writes a marker file; verify with `cargo test` that root search finds its command and the marker is absent

## 3. Commands and host API

- [x] 3.1 Implement `JsCommand`: start a session, call the export named by the command id, push views as `ViewTree`s from JSON, finish on return or reported failure; verify with `cargo test` for a loading-then-content view command, a no-view success with message, and a reported failure
- [x] 3.2 Map a missing export to "This command's extension has no code for it" and uncaught errors or rejections to "This command stopped because of an error, see dango.log" with the error, stack, and ids in the log; verify with `cargo test`
- [x] 3.3 Map out-of-memory to "This command used too much memory, so Dango stopped it" and end a session when its context is cancelled; verify with `cargo test` that a looping command ends after cancel and a pending timer never pushes a view
- [x] 3.4 Add the own `console` writing through `crate::log::append` with the extension id; verify with `cargo test` that `console.log("hi")` lands in the log with the id
- [x] 3.5 Add `dango.exec` on `platform::contained_command` with the agent PATH, killing the child when the session ends; verify with `cargo test` that a program's code and stdout come back and that a long-running program is gone after cancel
- [x] 3.6 Add `dango.fetch`, `dango.fs`, `dango.open`, `dango.copy`, `dango.paste`, `dango.env`; verify each with `cargo test` (fetch against a local `TcpListener` server, fs in a temp dir, open/copy/paste through the existing test fakes)
- [x] 3.7 Add the `local_extension_storage` migration and `dango.storage`; verify with `cargo test` that values survive a reopened database and that two extensions with the same key read their own values
- [x] 3.8 Allow extra keys in `extensions.<id>` in the config schema and pass them as `dango.preferences`; verify with `cargo test` that `user = "nico"` loads without a problem and reaches the extension, and that a reload changes the value for the next session
- [x] 3.9 Add `dango.secret` reading `[extensions.<id>]` from `auth.toml` per call, and make `AuthFile::key` ignore the `extensions` table; verify with `cargo test` for a found token, a missing table, another extension's secret, a provider key out of reach, an unparsable file logging only the line, and a provider named `extensions` getting no key

## 4. Actions

- [x] 4.1 Route `perform_action` for JS extensions to the session's handler table, block on the handler, and map its result to `Replaced`, `Ticker`, `Back`, `Done`, `CopyToClipboard`, or `Failed`; verify with `cargo test` for each mapping
- [x] 4.2 Push a new level through `Runner::start` and return `Started`, with the session following the newest context; verify with `cargo test` that a pushed detail view keeps the session alive and Escape back to the list keeps the list's actions working
- [x] 4.3 Deliver form values to the submit handler; verify with `cargo test` that "Fix login" reaches the handler for field "title"
- [x] 4.4 End a session when its newest context is cancelled; verify in `src/app/tests.rs` that Escape out of a JS view stops its timer and its running program

## 5. Cached root items

- [x] 5.1 Implement the cached `RootProvider` calling the exported `rootItems()` on load, enable, and reload; verify with `cargo test` that rows match a query and that ten keystrokes run no JS
- [x] 5.2 Add the `refreshMinutes` interval (stops on deactivate) and keep last rows on a failed build; verify with `cargo test` using tokio's paused time
- [x] 5.3 Run a root item's actions through a new session calling `onAction(itemId, actionId)`; verify with `cargo test` that Enter on root item `repo-1` calls `onAction` with `repo-1` and its first action's id

## 6. Live search

- [x] 6.1 Add the optional `filtering` field to `ListView` and `GridView` (default `"launcher"`, protocol stays 2); verify with `src/ui/tests.rs` that a tree without it filters as before
- [x] 6.2 In the `src/ui/` reducer, skip narrowing for `"command"` views and emit `SEARCH_ACTION` with the query on each change; drop a `Replaced` reply for an older query; verify with `src/ui/tests.rs` cases for delivery, out-of-order answers, no re-narrowing, the 50-item limit in command order, and query kept on going back
- [x] 6.3 Hand the query and an abort signal to the JS search handler; verify with `cargo test` that a superseded search's result is never pushed

## 7. Reload and types file

- [x] 7.1 Add `ExtensionHost::remove`; verify with a `cargo test` in `src/extension/mod.rs` that it deactivates, unregisters commands, and leaves other extensions alone
- [x] 7.2 Watch `extensions/` with `notify`, group events per folder, then `replace` registered ones, `register` unknown ones, `remove` deleted or broken ones, and rebuild candidates, pipeline, and hotkeys as `reload_config` does; verify with `cargo test` on a temp dir that editing `main.js` changes the next run, a folder fixed after a failed start loads, deleting a folder releases its hotkeys, and a broken manifest is reported while others keep working
- [x] 7.3 Add the `dango.d.ts` template (documenting `rootItems`, `onAction`, and that storage is machine-local) and the drift test that writes `assets/dango.d.ts`; write it at startup only when the folder exists; verify with `cargo test` that the checked-in file matches and that no folder means no file
- [x] 7.4 Update `AGENTS.md`: replace "sandboxed host" with the trusted JS host, add `src/js/` to Architecture, and add any build fact found in group 1; verify by reading the diff

## 8. Platform checks

- [x] 8.1 Windows: in a release build started with `DANGO_CONFIG_DIR` set to a scratch dir, run a fixture command that calls `dango.exec("gh", ...)`; check through `dango-ui` and `mise run shot` that no console window appears and the launcher keeps focus
- [x] 8.2 macOS: start the bundle the Finder way (`env -i ... open`, see `AGENTS.md`) and run a fixture command that execs a Homebrew program; check through `dango-ui` that its output shows

## 9. Review

- [x] 9.1 Run `mise run check` and `cargo test`; both pass
- [x] 9.2 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context; report findings to the author

## Workflow follow-up

- Archive with `/opsx:archive` once 8.2 has been done on the Mac.
