# Design

## Context

- **Root search** builds `ResultItem`s in `Core::result_item` from
  `Candidate`s. Root rows draw a title, subtitle, and icon, but no accessories
  (`render/list.rs` `result_row`).
- **Global hotkeys:** `hotkeys::Registry` keeps `(HotKey, Target)` for every
  chord that registered, with commands and applications as targets. The
  hyperkey has already been narrowed by then. Nothing formats a `HotKey` for
  display. `keys::shortcut_label` formats protocol `Shortcut`s, and shows Ctrl
  as ⌘ on macOS, which is wrong for a global hotkey.
- **AI commands** are `InvocationMode::View`: dispatching one, from root or its
  hotkey, shows the launcher first. `Inner::run` streams a detail view through
  the `&InvocationContext` that `Command::invoke` lends. It stops when
  `ctx.is_cancelled()`, which Escape, a newer invocation, a deliberate hide, or
  the hold running out make true. Then `deliver` shows, pastes (`ctx.succeed()`
  hides first), or copies. `start` (form submit) uses `runner.start()` for an
  owned `Arc<InvocationContext>`.
- **Background tasks** (`task-registry`):
  - The busy toast for a running task, with the owner's Cancel action.
  - Result toasts: on `Done` / `NeedsInput` / `Failed`, a toast at once; while
    hidden, the HUD for a failure and a toast on the next open.
  - The reducer's "own view" check is a progress view of the owner on top.

## Decisions

### Hotkey chords on root rows

- `ResultItem.hotkey: Option<HotKey>`, filled in `Core::result_item` by looking
  the candidate up in `services.hotkeys` (`Target::Command` for commands,
  `Target::Application` for application results). Only registered chords show,
  so a refused or conflicting one never does.
- New `keys::hotkey_label(&HotKey) -> String`: on macOS the modifiers in
  "⌃⌥⇧⌘" order then the key, on Windows "Ctrl+Alt+Shift+Win+" then the key.
  The key name comes from the `Code` (letters, digits, F-keys, named keys). A
  chord registered for the hyperkey shows the modifiers it is registered with,
  because that is what the operating system sees.
- `result_row` draws it as a key chord accessory on the right, through the
  existing `AccessoryData { kind: kbd }`, with the label string. Root items
  carry no other accessories, so there is no limit logic.

Rejected: parsing the config string for display. That repeats `mod`/`hyper`
and per-platform resolution, and would show chords that never registered.

### AI runs as background tasks

**When the task starts:** `run` starts the task after `prepare` returns a
request. A form waiting for arguments, or "Select some text first", never
starts one, and never cancels a running one. At that point, a running task of
another command is cancelled.

**Owned context:** the run keeps its view context in `Inner.live:
Mutex<Option<Arc<InvocationContext>>>`.
- Started from `invoke`, the lent context is wrapped. The invoke call blocks
  until the run ends, as it does today, so the context lives as long as the
  run.
- Started from `start` (form submit), it already owns one.
- Running the same command again while it runs: `invoke` swaps `Inner.live`
  for its own context, pushes the last streamed text into it, and returns. The
  first invoke, still blocking, keeps driving the run and pushes through
  `Inner.live`, which is the new view's context.

**Stopping:** the run stops on the task's cancel signal, not on
`ctx.is_cancelled()`. Dropping `Chunks` kills an agent subprocess
(`agent/mod.rs`).

**"Away":** the run is away when `Inner.live`'s context is cancelled (the view
was left, a newer invocation started, or the hold ran out) or the launcher is
hidden. Visibility comes from a `visible: Arc<dyn Fn() -> bool + Send + Sync>`
that Core hands the AI extension, reading its own shown state.

**Endings:**

| Where | Show / Paste | Copy | Failure |
|---|---|---|---|
| In view | as today (`deliver`); task ends `Cancelled`, which posts no toast | as today | `ctx.fail` as today; task ends `Cancelled` |
| Away | answer kept per command in `Inner.kept`; task ends `Done { "Answer ready: <title>" }` with `show` | copied; task ends `Done { "Copied to clipboard" }` | task ends `Failed { sentence }` (HUD and toast per background-tasks) |

Away but held (hidden, context still live), the view is also replaced by the
finished answer, or by a non-loading detail on failure, so the next open never
shows a dead loading view.

The answer is kept under the `Inner.live` lock, in the same step that ends the
run. Running the same command again checks the live run and then the kept
answer under that lock, so it finds one of them and never starts a second
request in between. A kept answer opens once, from the toast or from running
the command again, and is then dropped.

An in-view run ends without a result, so no stray or delayed toast appears.
Pasting in view already hides the launcher.

**Actions:**
- **Running view:** "Continue in background" (`background`, primary, returns
  `Back`) and "Cancel" (`cancel`, shortcut ⌘. / Ctrl+.). From the view, `cancel`
  returns `Back`; from the toast, nothing pops, as the update does.
- **`show`:** takes the kept answer and pushes it as the finished detail
  (`detail_tree(answer, false, true)`) through `runner.start()`.

**`TaskState.show: Option<String>`:** the reducer attaches it to a success
result toast as "Show answer" with `{ key: "Enter", modifiers: [Cmd] }`, which
is Ctrl+Enter on Windows and Cmd+Enter on macOS.

**No widening of the reducer's "own view" check.** An in-view AI run posts no
result. While it streams, its detail is `loading`, so the "Loading…" busy
source wins over the task source. The update keeps its progress-view rule. The
`background-tasks` delta says "its own view" for the spec. The code keeps
progress views as the update's own view and AI's own view never needs the
check.

Alternatives rejected:

- **A progress view for AI runs.** The answer is markdown that streams, and a
  progress view has no body.
- **Pasting when a run finishes away.** The user is somewhere else by then.
- **Widening "own view" to any view of the owner.** "Check for Updates" pushes
  plain details, which would hide the download's busy toast.

### Copy

Written with the ux-writing skill and checked against `interface-copy`.

| Where | String |
|---|---|
| AI busy toast | "Running <title>…" (the existing running toast's form) |
| AI result toast | "Answer ready: <title>" |
| Toast action | "Show answer" |
| Running view, primary | "Continue in background" (as the update's) |
| Running view, panel | "Cancel" |
| Copy finished away | "Copied to clipboard" (existing) |

### Contracts

There is no view-protocol or manifest change. `ResultItem`, `TaskState`, and
the AI extension's internals are internal.

### Stacking

This builds on `task-registry` and `inline-answer`. Its `background-tasks`
delta modifies requirements that exist only after `task-registry` is archived,
so archive this one last.

## Risks / Trade-offs

- **An AI run keeps costing tokens after the user leaves.** Cancel stops it,
  and another AI command replaces it.
- **"Show answer" lasts about two seconds**, as every success toast does. The
  user chose this over a lasting toast. After it goes, running the command
  again starts a new request.
- **Cmd+Enter / Ctrl+Enter** is also the form-submit chord. While the toast
  shows, its chord wins (per `task-registry`), so a form submit pressed in
  those two seconds runs "Show answer" instead.
- **A copy finished away writes the clipboard** while the user may be working
  elsewhere. The user chose this, and the toast says so.
