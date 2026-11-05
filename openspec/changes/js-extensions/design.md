# Design

## Context

See proposal.md for why. Today every extension is a native built-in:

- `Extension` (`src/extension/mod.rs`) gives a `Manifest`, `command(id)`
  returning an `invocation::Command`, `root_provider()`, `services()`, and a
  sync `perform_action(item, action, values) -> ActionOutcome`.
- `Command::invoke(&InvocationContext)` pushes `ViewTree`s through a sink and
  finishes with an `Outcome`. `ctx.is_cancelled()` tells it the launcher moved
  on.
- `registry::Host` has one variant, `Builtin`.
- `protocol.rs` types already derive serde in camelCase. `Manifest` has no
  serde (dropped in f4ff68b).
- The filter dropdown already sends a reserved action (`FILTER_ACTION`) to the
  view's command through `perform_action`. Live search reuses that path.
- Running programs: `platform::contained_command` (process-wrap with job object
  and `CREATE_NO_WINDOW` on Windows) and `agent_known_dirs` /
  `login_shell_env` build the PATH tray apps lack (`AGENTS.md`, platform traps).
- The config dir is `~/.config/dango/` (`src/config/location.rs`).

## Goals / Non-Goals

**Goals:**

- One embedded runtime, no extra process, no memory used by idle extensions.
- JS extensions plug into `ExtensionHost` as ordinary `Extension`s, so
  enable/disable, aliases, hotkeys, per-command `enabled`, ranking, and failure
  isolation work unchanged.
- Side effects (processes, HTTP, clipboard, open) go through the same Rust
  paths as built-ins, so the platform traps are handled once.

**Non-Goals:**

- Node compatibility beyond what falls out of the chosen modules. No `require`,
  no `node_modules` resolution, no `Buffer` shim in v1.
- A JS-side UI framework (JSX, React). Extensions build plain objects.
- A published npm package for the SDK. The SDK ships inside Dango.

## Decisions

### 1. Embedded QuickJS through `rquickjs` 0.14, async, one runtime per session

A **session** is one unit of work: an invocation (with the views it pushes and
the actions run in them), or one root items build. Each session gets a fresh
`AsyncRuntime` + `AsyncContext` (features `futures`, `parallel`, `loader`,
`macro`), loads `main.js` as an ES module, runs, and is dropped when the
session ends. Sessions run as tasks on the tokio runtime the shell already
owns.

- Memory: `set_memory_limit` per runtime (start at 64 MB, a constant in the
  host module). Exceeding it raises an out-of-memory error, which the host maps
  to the memory failure ticker.
- Runaway code: `set_interrupt_handler` returns true once the session's
  `InvocationContext` is cancelled, which stops endless loops at the next
  bytecode check. Dropping the task drops pending host futures (requests,
  processes) too.
- Async: host functions are Rust `async fn`s exposed as JS promises
  (`Promised`); `AsyncRuntime::idle` drives the job queue.
- Verified in rquickjs-core 0.14.0 source: `AsyncRuntime`, `AsyncContext`,
  `async_with!`, `Ctx::spawn`, `set_memory_limit`, `set_interrupt_handler`,
  `idle`, `Promised`, and the `parallel` feature exist. The first task is a
  spike that proves this on both platforms before anything else depends on it.

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

- `rquickjs-extra` 0.3 (built on rquickjs 0.14 and tokio): `timers`
  (`setTimeout`, `setInterval`), `console` (routed to the Dango log with the
  extension id), `url`.
- Own `dango` module, all async where I/O is involved:
  - `exec(program, args, { cwd, env, input })` returns
    `{ code, stdout, stderr }`. Built on `platform::contained_command` with the
    agent PATH. Runs through `spawn_blocking` because process-wrap is used
    with its `std` feature.
  - `fetch(url, init)` with a small `Response` (`status`, `headers`,
    `text()`, `json()`), on the `reqwest` client already in the tree.
  - `fs`: `readText`, `writeText`, `readDir`, `remove`, `exists`, `stat`,
    on `std::fs` in `spawn_blocking` (tokio's `fs` feature stays off).
  - `open(target)`, `copy(text)`, `paste(text)`: the opener and clipboard
    paths built-ins use.
  - `storage`: `get`, `set`, `remove`, `keys`, scoped to the extension id.
  - `preferences`: a frozen object read when the session starts.
  - `env`: the extension id and its folder path.
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
- **`rquickjs-extra` `os` and `sqlite`**: `os` is not needed; `sqlite` would
  give extensions a raw database, but storage is a scoped key-value store.
  Not enabled.

### 3. JS extension = `JsExtension: Extension`, host variant `Host::Js`

- Loader scans `<config_dir>/extensions/*/`, reads `manifest.json` into the
  existing `Manifest` through a separate serde DTO in the JS host module (so
  built-in manifests stay serde-free), and checks `id == folder name`, no
  `services`, then `Manifest::validate`.
- `JsExtension::command(id)` returns a `JsCommand` that starts a session and
  calls the export named by the command id. A missing export fails with the
  "can't run" ticker.
- Commands from JS extensions register with `Host::Js`; nothing else in the
  registry changes.
- Manifest `commands` from the file are the declared set; config entries
  under `extensions.<id>.commands.<cmd>` give `enabled`, alias, and hotkey as
  for built-ins.

### 4. Actions: closures while the session lives, `onAction` otherwise

`perform_action` is sync and returns an `ActionOutcome`. A JS action returns
`ActionOutcome::Started` and reports through the session's
`InvocationContext` (push, replace, ticker, back, done, fail), which already
exists for built-ins that stream.

- Inside a view, the SDK gives each action an id and keeps its handler in a
  table owned by the live session. Dango sends `(item_id, action_id, values)`
  back; the SDK calls the handler.
- Root item actions have no live session. Dango starts one and calls the
  exported `onAction(itemId, actionId)`. The SDK documents this in
  `dango.d.ts`.
- A session ends when its invocation is superseded or its last view leaves
  the stack. This is the existing `is_cancelled` signal.

Alternative: **declarative native effects on actions** (`open`, `copy` done by
Dango without JS). Faster for root items, but it adds fields to the protocol
that only JS uses, and starting a QuickJS runtime is cheap. Rejected for v1.

### 5. Cached root items as a native provider

`JsExtension::root_provider()` returns a Rust `RootProvider` that holds the
last rows in memory and answers `items(query)` with no JS. Rows map to
`search::Candidate`. A build runs on load, enable, reload, and on a
`refreshMinutes` tokio interval while enabled. A failed build keeps the last
rows and logs. The interval task ends on deactivate, so it isn't a service
from the user's point of view: it only builds rows, then the runtime goes
away.

Alternative: **live root items** (JS per keystroke). Out of scope by decision;
it would keep runtimes alive and put JS on the root search path.

### 6. Live search in the protocol (**view protocol contract change**)

- `ListView` and `GridView` gain `filtering: "launcher" | "command"`, default
  `"launcher"`. `PROTOCOL_VERSION` becomes 3, since a version 2 build would
  read a `"command"` view wrongly and filter it itself. Built-ins only change
  the constant.
- With `"command"`, the `src/ui/` reducer skips its own narrowing for that
  view and emits a reserved `SEARCH_ACTION` (beside `FILTER_ACTION`) with the
  query on every change. The item limit still applies in the command's order.
- Each `SEARCH_ACTION` carries the invocation counter; the core drops a tree
  whose query is not the latest, and the session's JS sees the previous
  search's abort signal. No debounce, as `AGENTS.md` performance requires.
- Per-level query keeping works as today because the query stays on the view
  stack level.

### 7. Manifest stays at version 1 (**manifest contract**)

Alpha with no users, so the JSON shape is v1. It adds `rootItems` as an object
with an optional `refreshMinutes`, which the Rust `Manifest` represents as
`root_items: bool` plus the refresh value in the JS host's own DTO. Built-in
manifests are untouched.

### 8. Storage in the local database

One machine-local table `extension_storage(extension_id, key, value_json,
updated_at)`, primary key `(extension_id, key)`, added by a migration. Machine-
local, so no UUID or soft delete (see `openspec/specs/local-store`). Values
stay when an extension is removed, so reinstalling it keeps them.

### 9. Preferences in `extensions.<id>`

The config schema for `extensions.<id>` sets `additionalProperties: true`
besides `enabled` and `commands`. The host reads the extra keys as a JSON
object per extension on each config load; a session gets the snapshot from
when it started.

### 10. Hot reload through `notify`

One watcher on `extensions/` (recursive). Events are grouped per top-level
folder and settled briefly, as `refresh.rs` does for installs. A changed
folder goes through `ExtensionHost::replace`; a new folder through
`register`; a removed one is unregistered. Open sessions keep their loaded
code until they end. `dango.d.ts` changes are ignored.

### 11. `dango.d.ts` generated with `ts-rs`, checked in

- `ts-rs` 12 (default `serde-compat`) derives TypeScript for the
  `protocol.rs` types and the manifest DTO, as a dev-dependency under a
  `cfg_attr(test, derive(TS))`.
- A test concatenates the generated declarations with a hand-written template
  for the `dango` module and SDK helpers, and compares the result with the
  checked-in `assets/dango.d.ts`. With an env var set, it rewrites the file.
  So the shipped file never drifts from the Rust types.
- Dango writes the file with `include_str!` at startup when the folder exists
  and the content differs.
- Not checked yet: how ts-rs renders our `#[serde(tag = "kind")]` enums and
  `#[serde(default)]` fields. Covered by the spike task. Fallback: `specta`
  2.0 (still a release candidate, rc.28), else hand-written types with the
  same drift test against `serde_json` samples.

### 12. Failure reporting

- Load problems join `LoadReport` and reach the tray through `load_problems`
  in `src/app/mod.rs`.
- Uncaught errors and rejections: message, stack, extension id, and command id
  to the log; generic ticker to the user.

### User-visible strings

Written with the ux-writing skill and checked against
`openspec/specs/interface-copy`:

| Where | String |
|---|---|
| Tray, extension failed to load | "Extension <id> didn't load, see dango.log" |
| Failure ticker, command has no code | "This command can't run. Its extension has no code for it." |
| Failure ticker, uncaught error | "This command stopped because of an error, see dango.log" |
| Failure ticker, memory limit | "This command used too much memory, so Dango stopped it." |

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
  → Spike first (task 1), on Windows, before other groups. Fallback: a
  dedicated thread per session running a sync `Runtime`, with host calls
  blocking on the tokio handle.
- [QuickJS C build adds compile time and needs a C compiler on both machines]
  → MSVC is already present for Rust on Windows, clang on macOS. Note it in
  `AGENTS.md` build facts if it bites.
- [Most npm packages won't run (no Node built-ins)] → Accepted. Authors bundle
  pure-JS dependencies with esbuild. Revisit a Node shim only when a real
  extension needs it.
- [Trusted extensions can do anything the user can] → Accepted by decision.
  `AGENTS.md` says so; sharing means reading the code.
- [A tight loop inside a long host call can't be interrupted] → Host calls are
  async and dropped with the task; only JS bytecode needs the interrupt
  handler.
- [Live search floods the command] → Each new query cancels the last, so at
  most one search runs per view.
- [Protocol version 3 breaks any tree at version 2] → Only built-ins produce
  trees today; they read the constant.

## Migration Plan

None. No existing JS extensions, alpha stage. `AGENTS.md` gets the trusted JS
host in place of "sandboxed host" in the same change.
