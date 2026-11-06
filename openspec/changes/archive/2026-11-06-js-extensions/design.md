# Design

## Context

See proposal.md for why. Today every extension is a native built-in:

- `Extension` (`src/extension/mod.rs`) gives a `Manifest`, `command(id)`
  returning an `invocation::Command`, `root_provider()`, `services()`, and a
  sync `perform_action(item, action, values) -> ActionOutcome`.
- `perform_action` already runs off the main thread: `run_effect` hands
  `Effect::RunAction` to `spawn_blocking` (`src/app/mod.rs`), and the reducer
  applies the returned `ActionOutcome` (`action_finished` in
  `src/ui/state.rs`): `Replaced`, `Removed`, `Ticker`, `Failed`, `Back`,
  `Done`, `CopyToClipboard`, `Started`.
- `Command::invoke(&InvocationContext)` pushes `ViewTree`s and finishes with
  an `Outcome`. A tree from the invocation already shown replaces the top
  level; a tree from a new invocation pushes a level. `Runner::start`
  (`ViewRunner`) opens such a new invocation for work an action begins.
  `ctx.is_cancelled()` turns true when the launcher hides, Escape leaves the
  view, or a newer invocation starts.
- `protocol.rs` types already derive serde in camelCase. `Manifest` has no
  serde (dropped in f4ff68b).
- The filter dropdown sends the reserved `FILTER_ACTION` through
  `perform_action`. Live search reuses that path.
- Running programs: `platform::contained_command` (process-wrap, job object,
  `CREATE_NO_WINDOW` on Windows) and `agent_known_dirs` / `login_shell_env`
  build the PATH a tray app lacks (`AGENTS.md`, platform traps). The AI agent
  kills its child explicitly on cancel (`src/extensions/ai/agent/mod.rs`).
- Secrets: `AuthFile` (`src/extensions/ai/auth.rs`) reads `auth.toml` per
  request and never puts file text into errors.
- The config dir is `~/.config/dango/` (`src/config/location.rs`). The config
  schema rejects unknown keys under `extensions.<id>` today
  (`src/config/schema.rs`).
- Dango's log is `crate::log::append`; there is no `log` crate.

## Goals / Non-Goals

**Goals:**

- One embedded runtime, no extra process, no memory used by idle extensions.
- JS extensions plug into `ExtensionHost` as ordinary `Extension`s, so
  enable/disable, aliases, hotkeys, per-command `enabled`, ranking, and failure
  isolation work unchanged.
- Side effects (processes, HTTP, clipboard, open, secrets) go through the same
  Rust paths as built-ins, so the platform traps are handled once.

**Non-Goals:**

- A JS-side UI framework (JSX, React). Extensions build plain objects.
- A published npm package for the SDK. The SDK ships inside Dango.

## Decisions

### 1. Embedded QuickJS through `rquickjs` 0.14, async, one runtime per session

A **session** is one unit of work: an invocation together with the views it
and its actions show, or one root items build. Each session gets a fresh
`AsyncRuntime` + `AsyncContext` (features `futures`, `parallel`), loads `main.js` as an ES module, runs, and is dropped when the
session ends. Sessions run as tasks on the tokio runtime the shell owns.

- Memory: `set_memory_limit` per runtime (start at 64 MB, a constant in
  `src/js/`). Exceeding it raises an out-of-memory error, which maps to the
  memory failure ticker.
- Runaway code: `set_interrupt_handler` returns true once the session is
  cancelled, which stops endless loops at the next bytecode check. Ending the
  session drops pending host futures (requests, timers) and kills programs it
  started (decision 2).
- Async: host functions are Rust `async fn`s exposed as JS promises
  (`Promised`); `AsyncRuntime::idle` drives the job queue.
- Verified in rquickjs-core 0.14.0 source: `AsyncRuntime`, `AsyncContext`,
  `async_with!`, `Ctx::spawn`, `set_memory_limit`, `set_interrupt_handler`,
  `idle`, `Promised`; `Send`/`Sync` for `AsyncRuntime` are behind `parallel`.
  The first task group is a spike on Windows that proves this before anything
  depends on it. macOS gets the CI compile check and the hand check in task
  8.2, per the one-platform-per-session rule.

Alternatives:

- **Raycast-style Node or Bun child process**: full npm, but a second runtime
  to ship or require, tens of MB per process, a stdio protocol to design.
  Rejected for weight; the author's use is small integrations.
- **Script commands** (any executable printing JSON): the cheapest option, but
  no callbacks, no live search, no actions that change a view. Rejected
  because the decided scope needs those.
- **`deno_core` (V8)**: npm-grade compat, but a large binary, long builds, and
  V8 inside the Dango process. Rejected.
- **WASM (wasmtime, extism)**: sandboxed and any language, heavier and more
  friction for authors. Rejected; sandboxing is a non-goal.
- **Rhai**: pure Rust and tiny, but a niche language for authors. Rejected.
- **One long-lived VM per extension**: simpler action routing, but holds
  memory while idle, which breaks a stated goal. Rejected.

### 2. Host modules: own `dango` module plus `rquickjs-extra`

- `rquickjs-extra` 0.3 (built on rquickjs 0.14 and tokio), features `timers`
  (`setTimeout`, `setInterval`) and `url`.
- Own `dango` module, async where I/O is involved:
  - `exec(program, args, { cwd, env, input })` returns
    `{ code, stdout, stderr }`. Built on `platform::contained_command` with the
    agent PATH, run in `spawn_blocking` (process-wrap uses its `std` feature).
    The session keeps the child handle and kills it when the session ends, as
    the AI agent does; dropping a `spawn_blocking` task alone would let the
    program run on.
  - `fetch(url, init)` with a small `Response` (`status`, `headers`,
    `text()`, `json()`), on the `reqwest` client already in the tree.
  - `fs`: `readText`, `writeText`, `readDir`, `remove`, `exists`, `stat`, on
    `std::fs` in `spawn_blocking` (tokio's `fs` feature stays off).
  - `open(target)`, `copy(text)`, `paste(text)`: the opener and clipboard
    paths built-ins use.
  - `storage`: `get`, `set`, `remove`, `keys`, scoped to the extension id.
  - `secret(name)`: decision 13.
  - `preferences`: a frozen object read when the session starts.
  - `env`: the extension id and its folder path.
- Own `console` object (`log`, `warn`, `error`) writing through
  `crate::log::append` with the extension id. A few lines, and it avoids
  adding the `log` crate plus an adapter that `rquickjs-extra`'s `console`
  would need.
- Values cross the boundary as JSON strings (`JSON.stringify` in the SDK,
  `serde_json` in Rust), so `protocol.rs` types are reused as they are and no
  rquickjs-serde bridge is needed.

Alternatives:

- **`llrt_modules`** (AWS LLRT, Node-compatible `fs`, `child_process`,
  `fetch`, `buffer`): checked 0.8.1-beta. It pins rquickjs 0.11 (current is
  0.14) and is beta. Its `child_process` can't take Dango's job object,
  `CREATE_NO_WINDOW`, or PATH fix, so it would show console windows in
  release builds and miss Homebrew tools on macOS. Its `fetch` adds a second
  HTTP stack beside reqwest. Rejected.
- **`rquickjs-extra` `console`**: logs through the `log` facade, which Dango
  doesn't use. Rejected for the own `console` above.
- **`rquickjs-extra` `os` and `sqlite`**: `os` is not needed; `sqlite` would
  give extensions a raw database, but storage is a scoped key-value store.
  Not enabled.

### 3. JS extension = `JsExtension: Extension`

- Loader scans `<config_dir>/extensions/*/`. A folder with neither file is
  ignored; one with only one of them is reported. It reads `manifest.json`
  into the existing `Manifest` through a separate serde DTO in `src/js/` (so
  built-in manifests stay serde-free), resolves a relative `icon` against the
  folder, and checks `id == folder name`, no `services`, then
  `Manifest::validate`.
- `JsExtension::command(id)` returns a `JsCommand` that starts a session and
  calls the export named by the command id. A missing export fails with the
  "no code" ticker.
- No new `registry::Host` variant. Nothing outside tests reads `.host`, so a
  variant would need a hook with no consumer. Add one when something needs to
  tell hosts apart.
- Manifest `commands` are the declared set; config entries under
  `extensions.<id>.commands.<cmd>` give `enabled`, alias, and hotkey as for
  built-ins (that plumbing already works).

### 4. Actions: block on the JS handler, push through `Runner::start`

`perform_action` runs in `spawn_blocking`, so a JS action can block on its
handler's promise there and map the result onto the existing `ActionOutcome`:

| JS handler result | `ActionOutcome` |
|---|---|
| returns a view to replace the current one | `Replaced` |
| returns a ticker message | `Ticker` |
| returns "back" | `Back` |
| returns nothing / "done" | `Done` |
| returns text to copy | `CopyToClipboard` |
| reports a failure, throws, rejects | `Failed` (uncaught: generic ticker) |
| pushes a new view | `Runner::start` → new context → `Started` |

- Inside a view, the SDK gives each action an id and keeps its handler in a
  table owned by the live session. Dango sends `(item_id, action_id, values)`;
  the SDK calls the handler. A form submit is an action with `values`.
- Pushing a level needs a new invocation, so the session asks
  `Runner::start` for a new `InvocationContext` and pushes through it. The
  session then follows the newest context it was handed: it ends when that
  context is cancelled (launcher hidden, Escape below the session's first
  level, another command run), not when its first invocation is superseded by
  its own push.
- Root item actions have no live session. Dango starts one and calls the
  exported `onAction(itemId, actionId)`; the result maps the same way.

Alternative: **declarative native effects on actions** (`open`, `copy` done by
Dango without JS). Faster for root items, but it adds protocol fields only JS
uses, and starting a QuickJS runtime is cheap. Rejected for v1.

### 5. Cached root items as a native provider

`JsExtension::root_provider()` returns a Rust `RootProvider` that holds the
last rows in memory and answers `items(query)` with no JS. Rows map to
`search::Candidate`. A build calls the exported `rootItems()` on load, enable,
reload, and on a `refreshMinutes` tokio interval while enabled. A failed build
keeps the last rows and logs. The interval task ends on deactivate.

Alternative: **live root items** (JS per keystroke). Out of scope by decision;
it would keep runtimes alive and put JS on the root search path.

### 6. Live search in the protocol (**view protocol contract change**)

- `ListView` and `GridView` gain an optional `filtering: "launcher" |
  "command"`, default `"launcher"`. `PROTOCOL_VERSION` stays 2: by the rule in
  `protocol.rs`, a bump is for trees that could be read wrongly, and a tree
  without the field reads as before.
- With `"command"`, the `src/ui/` reducer skips its own narrowing for that
  view and emits a reserved `SEARCH_ACTION` (beside `FILTER_ACTION`) with the
  query on every change. The 50-item limit applies in the command's order (the
  view-protocol delta modifies that requirement).
- The request carries the query. The command answers with
  `ActionOutcome::Replaced`, not a push: after Escape every earlier invocation
  context is superseded, so pushes into the list below would be dropped. The
  reducer drops a reply whose query is no longer the view's query, and the
  session's JS sees the previous search's abort signal. No debounce, as
  `AGENTS.md` performance requires.
- One search handler per session: the latest view declaring `onSearch`.
- Per-level query keeping works as today because the query stays on the view
  stack level.

### 7. Manifest stays at version 1 (**manifest contract**)

Alpha with no users, so the JSON shape is v1. It adds `rootItems` as an object
with an optional `refreshMinutes`, which the Rust `Manifest` represents as
`root_items: bool` plus the refresh value in the JS host's DTO. Built-in
manifests are untouched.

### 8. Storage in the local database

One table `local_extension_storage(extension_id, key, value_json,
updated_at)`, primary key `(extension_id, key)`, added by a migration, with
the `local_` prefix machine-local tables use. It is machine-local because
sync doesn't exist and an extension's data is tied to this machine's setup.
Unlike the app index it can't be rebuilt; extensions should keep only what
they can fetch again or lose without harm, and `dango.d.ts` says so. Values
stay when an extension is removed, so putting it back keeps them.

### 9. Preferences in `extensions.<id>`

The config schema for `extensions.<id>` allows extra keys besides `enabled`
and `commands`. The host reads them as a JSON object per extension on each
config load; a session gets the snapshot from when it started.

### 10. Hot reload through `notify`

One watcher on `extensions/` (recursive). Events are grouped per top-level
folder and settled briefly, as `refresh.rs` does for installs. Then:

- folder of a registered extension changed → `ExtensionHost::replace`
- folder not registered (new, or failed at startup and now fixed) →
  `ExtensionHost::register`; `replace` can't add an id it doesn't know
- folder removed or now broken → a new `ExtensionHost::remove`, which
  deactivates and drops the extension and its commands
- afterwards, rebuild search candidates, the root pipeline, and hotkeys the
  way `reload_config` in `src/app/mod.rs` does.

Open sessions keep their loaded code until they end. `dango.d.ts` changes are
ignored.

### 11. `dango.d.ts` generated with `ts-rs`, checked in

- `ts-rs` 12 (default `serde-compat`, handles `#[serde(tag)]`) derives
  TypeScript for the `protocol.rs` types and the manifest DTO, as a
  dev-dependency under `cfg_attr(test, derive(TS))`.
- A test concatenates the generated declarations with a hand-written template
  for the `dango` module and SDK helpers, and compares the result with the
  checked-in `assets/dango.d.ts`. With an env var set, it rewrites the file.
- Dango writes the file with `include_str!` at startup when the folder exists
  and the content differs.
- ts-rs makes a `#[serde(default)]` field optional only when it also skips
  serializing, since it can't tell input from output. `protocol_types!` adds
  `ts(optional_fields)` for `Option` fields, and each defaulted non-`Option`
  field carries `ts(optional, as = "Option<_>")`, all under `cfg_attr(test)`.
  `specta` (still rc.28) was the fallback and is not needed.

### 12. Failure reporting

- Load problems join `LoadReport` and reach the tray through `load_problems`
  in `src/app/mod.rs`.
- Uncaught errors and rejections: message, stack, extension id, and command id
  to the log; generic ticker to the user.

### 13. Secrets from `auth.toml`

- `dango.secret(name)` reads `auth.toml` on every call through the same
  reader as `AuthFile`, looking only at `extensions.<id>.<name>` for the
  calling extension. Returns `undefined` when the file, table, or name is
  missing; rejects with a "can't read auth.toml" error the extension can
  catch when it doesn't parse, and logs only the line (`config::line_of`).
- `AuthFile::key` ignores the `extensions` table, so no provider resolves to
  it.
- Dango never logs or shows a secret's value. What extension code does with
  it is the extension's business; extensions are trusted.

Alternative: **extension secrets in `local_extension_storage`**. No file to
edit, but plain text in the database, which the `ai-providers` rules forbid
for keys, and no way to set it without code. Rejected.

### User-visible strings

Written with the ux-writing skill and checked against
`openspec/specs/interface-copy`. One sentence each, no trailing period, like
existing tickers ("That action isn't available"):

| Where | String |
|---|---|
| Tray, extension failed to load | "Extension <id> didn't load, see dango.log" |
| Failure ticker, command has no code | "This command's extension has no code for it" |
| Failure ticker, uncaught error | "This command stopped because of an error, see dango.log" |
| Busy ticker, an action runs | "Running <action title>…", as for a command |
| Failure ticker, memory limit | "This command used too much memory, so Dango stopped it" |

The tray line follows the existing tray problems ("Hyperkey didn't start, see
dango.log"). Messages an extension reports itself are shown as given; the
interface-copy rules apply to Dango's own text only.

### UI surfaces

No new component. JS views render through the existing views in `ui/views/`
(list, grid, detail, form, progress), the SearchBar, ActionBar, and action
panel in `ui/shell/`, and the tickers. Live search changes only what the
reducer does with the query, not any component.

## Risks / Trade-offs

- [rquickjs async plus `parallel` behaves differently than the source suggests]
  → Spike first (group 1) on Windows. Fallback: a dedicated thread per session
  running a sync `Runtime`, with host calls blocking on the tokio handle.
- [QuickJS C build adds compile time and needs a C compiler on both machines]
  → MSVC is already present for Rust on Windows, clang on macOS. Note it in
  `AGENTS.md` build facts if it bites.
- [Most npm packages won't run (no Node built-ins)] → Accepted. Authors bundle
  pure-JS dependencies with esbuild. Revisit a Node shim only when a real
  extension needs it.
- [Trusted extensions can do anything the user can, secrets included] →
  Accepted by decision. `AGENTS.md` says so; sharing means reading the code.
- [A JS handler blocks a `spawn_blocking` thread while it awaits] → One thread
  per running action; actions are user-paced, so the pool never fills.
- [Live search floods the command] → Each new query cancels the last, so at
  most one search runs per view.

## Migration Plan

None. No existing JS extensions, alpha stage. `AGENTS.md` gets the trusted JS
host in place of "sandboxed host" in the same change.
