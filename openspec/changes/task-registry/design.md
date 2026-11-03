# Design

## Context

The updates extension (`src/extensions/updates/mod.rs`) already runs its
download apart from the view:

- `start_install` grabs an `InvocationContext`.
- It spawns `install`, which pushes a detail view with markdown "X of Y" on
  each twentieth of the download.
- It keeps the last tree in `shown`, so a re-opened launcher gets it back.
- After the download it calls `source.install`, which installs and restarts.

The updater (`cargo-packager-updater` 0.2.3) streams in `spawn_blocking`.
`download_extended(on_chunk, on_download_finish)` calls `on_download_finish`
after the last byte and before `verify_signature` (checked in the vendored
`src/lib.rs`, around line 753). So "Download" and "Verify signature" can be
told apart.

The reducer gets work's results as `Event`s and asks for work with `Effect`s.
Views arrive as `Event::Rendered`. Toasts are `HeldToast` (success, failure)
or derived busy text. The busy-mark machine from `progress-atoms` delays and
holds the busy toast. Key chords reach the reducer only if they are in the
snapshot's `bindings`.

## Goals / Non-Goals

**Goals:**

- One registry for long jobs, small enough that the update is its only user
  and adopting a second job is a few lines.
- The progress view is a protocol view kind built from the task's state, so
  the view and the toast read the same numbers.
- Toast actions as a general toast feature, with Retry and Cancel as the first
  users.

**Non-Goals:**

- Moving indexing or AI onto tasks (adopt-components).
- Speed and time left, persistence, and a list of all tasks.

## Decisions

### The registry: `src/tasks.rs`

```rust
pub struct Tasks { inner: Mutex<Vec<Entry>>, notify: Box<dyn Fn(TaskState) + Send + Sync> }
pub struct TaskHandle { id: u64, tasks: Arc<Tasks>, cancel: watch::Receiver<bool> }
pub struct TaskState {
    pub id: u64, pub key: String, pub title: String, pub line: String,
    pub status: TaskStatus,        // Running, NeedsInput, Done, Failed { retry: Option<RetryAction> }, Cancelled
    pub progress: Option<f32>,     // None = indeterminate
    pub steps: Vec<Step>,          // protocol::Step { title, status }
    pub detail: String,
}
```

- **Start:** `tasks.start(key, title)` returns `Err(existing id)` when a task
  with that key runs. The command then shows the running task's view again
  ("Coming back to a running task").
- **Report:** `handle.report(|state| ...)` changes the state. Each change goes
  through `notify`. Core sends it to the reducer as `Event::Task(TaskState)` on
  the main thread, as it sends rendered views today.
- **Cancel:** `handle.cancelled()` is a `tokio::sync::watch` receiver that
  work can `select!` on. tokio's `sync` feature is already on. `tokio-util`'s
  `CancellationToken` would also do, but it is a new dependency for one bool.
- **End:** a `Done`, `Failed`, or `Cancelled` status removes the entry from the
  registry after the reducer has been told. `NeedsInput` stays.

The registry lives in `Services`. It is handed to extensions the same way the
`ViewRunner` is.

### The progress view is a protocol view built from the state

```rust
View::Progress(ProgressView {
    title, subtitle, value: Option<f32>, detail,
    steps: Vec<Step { title, status: Status }>, actions,
})
```

`Status` is the accessory enum from `progress-atoms`. A helper,
`tasks::progress_view(&TaskState, actions) -> ViewTree`, builds it. The update
pushes it through its `InvocationContext` on each report, as it pushes the
detail view today. So the view is one more protocol view: no special path in
the reducer, and a third-party extension could emit one.

This extends the versioned view protocol. It stays at version 2: the change is
additive, and every existing tree stays valid.

Rejected:

- **The reducer draws the progress view straight from `Event::Task`.** That
  needs a new screen kind outside the view stack, and Escape, back, panel and
  bindings would all need a second path.
- **A `task` id inside the protocol view**, which the reducer would resolve.
  That leaks Core ids into the protocol.

### The reducer: tasks, background toast, result toast

The reducer keeps `tasks: Vec<TaskSummary { id, line, status, started }>`
from `Event::Task`.

- **Background toast:** the busy input becomes `working`, then top view
  loading, then the oldest running task, but only while the top view is not a
  progress view. The busy-mark timing applies as is. The marks remember which
  source they show. `ViewScreen.busy`, which drives the SearchBar bar, is true
  only for the first two sources, so a running task never draws a loading bar
  in some other view. When the source is a task, the busy toast carries a
  Cancel action.
- **Result toast:** if `Event::Task` ends a task while `visible` is false, the
  reducer holds it as `pending_result`. On `Event::Activated` it becomes a held
  toast, and a success expires as usual.

  If the launcher is shown and the top view is not that task's progress view,
  the result shows as a toast at once. "That task's" means the top view is a
  progress view pushed by the same owner. Only the update pushes one, so owner
  equality is enough.

  A failure carries Retry when the task named one.
- **Hidden failures:** these no longer go to the HUD for tasks. The HUD stays
  for no-view commands, as today.

### Toast actions

```rust
pub struct Toast { style, text, action: Option<ToastAction> }
pub struct ToastAction { title: String, shortcut: Shortcut, run: ToastRun }
pub enum ToastRun { Action(ActionRequest), CancelTask(u64) }
```

While a toast with an action shows, its chord is added to the snapshot's
`bindings`, and the reducer checks it before the view's own bindings.

- `Action` becomes `Effect::RunAction`.
- `CancelTask` becomes `Effect::CancelTask { id }`, which Core sends to the
  registry.

Either way the toast clears, and the key does nothing else. The ActionBar draws
the label and a `Kbd` after the text. Its accessible description gets the
action's label and chord.

Chords:

- **Cancel:** `{ key: ".", modifiers: [Cmd] }`. That is Cmd on macOS and Ctrl
  on Windows (`keys.rs` maps `Cmd` to the platform's command key). No current
  binding uses it.
- **Retry:** `{ key: "r", modifiers: [Cmd] }`. That is Ctrl+R on Windows, which
  "Show in Explorer" also uses for an app at root (`applications/mod.rs`). The
  toast wins while it shows. The spec says the chord SHALL NOT act on the view.
  Once the toast is gone, Ctrl+R reveals again.

### The update on a task

```
Install update --> tasks.start("dango.updates", "Dango 0.5.0")
   step "Check for updates" done (the item exists, so the check ran)
   step "Download" working, progress = received / total
   on_download_finish --> step "Verify signature" working, progress None
   Ok(bytes) --> step "Install and restart" needs input, line "Ready to restart"
                 bytes kept in Inner::ready, root item becomes "Restart now"
   Restart now --> source.install(bytes)   (quits on Windows, restarts on macOS)
   Err --> status Failed { retry: Install update }, step failed, detail = sentence
   cancelled --> stop waiting, drop the result, status Cancelled
```

- `UpdateSource::download` gains an `on_finished` callback, passed through as
  `on_download_finish`. `install` already exists, and is now called only from
  "Restart now".
- Cancel can't stop the blocking download thread: the updater has no abort.
  On cancel, `install` stops awaiting it (`select!` on the watch), drops the
  join handle, and reports `Cancelled`. The thread finishes its transfer and
  its bytes are dropped. The user sees the cancel at once, and nothing is
  installed.
- The `installing` flag and the `shown` tree go: the registry's "already
  running" answer and the task state replace them.

ActionBar per status in the progress view:

| Status | Enter | Panel |
|---|---|---|
| Running | "Continue in background" (pops the view) | "Cancel" ⌘. |
| Needs input | "Restart now" | "Install later" |
| Failed | "Retry" | - |

### UI parts

| Surface | Design part | Where |
|---|---|---|
| Progress view | "ProgressView view" | `ui/views/progress-view.slint` (new): title + muted subtitle, `Progress { kind: bar }` 4 px, muted detail, step rows with `Dot` (not announced: the step's text and status form its label, e.g. "Download, working") |
| Toast action | "Toast · failure Retry ⌘R" | `ui/shell/action-bar.slint`: label + `Kbd` after the text |
| Background toast | "Background task · Toast in ActionBar" | existing busy toast with spinner |

No existing view fits the progress view: a detail renders markdown and can't
hold a bar or status dots. A new `ProgressData` global feeds it, with one
writer, `src/slint_shell/render/progress.rs`. A progress view takes the
launcher's maximum height, as a detail does (`render/height.rs`).

### Copy

Written with the ux-writing skill and checked against `interface-copy`.
Action labels start with a verb and are at most four words.

| Where | String |
|---|---|
| Update item, idle | "Install update" |
| Update item, downloading | "Show progress" |
| Update item, ready (title) | "Dango 0.5.0 is ready to install" |
| Update item and progress view, ready | "Restart now" |
| Progress view, ready, panel | "Install later" |
| Progress view, running, Enter | "Continue in background" |
| Any running task | "Cancel" |
| Failure toast action | "Retry" |
| Progress view title | "Dango 0.5.0" |
| Progress view subtitle | "You have 0.4.2" |
| Detail, downloading | "18.4 MB of 29.6 MB", or "18.4 MB" without a total |
| Detail, verifying | "Checking the signature" |
| Detail, ready | "Ready to restart" |
| Steps | "Check for updates", "Download", "Verify signature", "Install and restart" |
| Background toast | "Downloading Dango 0.5.0 · 42%", or "Downloading Dango 0.5.0" |
| Result toast, ready | "Dango 0.5.0 is ready to install" |
| Step accessible label | "<step>, <status word>", for example "Download, working" |

The failure sentences stay as they are today.

## Risks / Trade-offs

- **A cancelled download keeps transferring** in its blocking thread until it
  ends, because the updater has no abort. Bandwidth only: the bytes are
  dropped and nothing installs.
- **About 30 MB stays in memory** while an update waits as ready. Acceptable
  for one download. "Install later" keeps it until Dango quits.
- **Ctrl+R overlap.** "Show in Explorer" is blocked while a failure toast with
  Retry shows. Any other key press clears the toast first.
- **The update no longer restarts on its own.** A user who confirms and walks
  away comes back to "Ready to restart", which is the point of the change.
