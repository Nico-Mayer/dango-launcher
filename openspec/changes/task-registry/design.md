# Design

## Context

The updates extension (`src/extensions/updates/mod.rs`) already runs its
download apart from the view:

- `start_install` grabs an `InvocationContext` and spawns `install`.
- `install` pushes a detail view with markdown "X of Y", throttled by
  `progress_step` to one push per twentieth of the download.
- It keeps the last tree in `shown`, so a re-opened launcher gets it back.
- After the download it calls `source.install`, which installs and restarts.

`cargo-packager-updater` 0.2.3 verifies the signature inside
`download_extended`, right after the last byte. It reports `UpdateError::
Download` and `UpdateError::Signature` apart. On Windows, `install` exits
the process. On macOS it swaps the bundle and returns.

The reducer gets work's results as `Event`s and asks for work with `Effect`s.
A view action goes `Effect::RunAction`, then `perform_action`, then an
`ActionOutcome` (`src/extension/mod.rs`). There is no outcome that pops one
view. `Launcher::intent` calls `clear_failure()` for every key before the key
is handled. Key chords reach the reducer only if they are in the snapshot's
`bindings`. Views from a running context reach the reducer through `pump`,
which calls `core.apply` from tokio.

## Goals / Non-Goals

**Goals:**

- A registry small enough that the update is its only user, and a second job
  is a few lines.
- The progress view is a protocol view built from the task's state, so the
  view and the toast read the same numbers.
- Toast actions as a general toast feature, with Retry and Cancel as the first
  users.

**Non-Goals:**

- Moving indexing or AI onto tasks (adopt-components).
- Speed and time left, persistence, and a list of all tasks.

## Decisions

### The registry: `src/tasks.rs`

```rust
pub struct Tasks { inner: Mutex<Vec<TaskState>>, core: OnceLock<Weak<Core>> }
pub struct TaskHandle { id: u64, tasks: Arc<Tasks>, cancel: watch::Receiver<bool> }
pub struct TaskState {
    pub id: u64, pub owner: String, pub title: String, pub line: String,
    pub status: TaskStatus,     // Running, NeedsInput, Done, Failed { sentence }, Cancelled
    pub progress: Option<f32>,  // None = indeterminate
    pub steps: Vec<Step>,       // protocol::Step { title, status }
    pub detail: String,
    pub cancel: Option<String>, pub retry: Option<String>,  // owner's action ids
}
```

- **Start:** `tasks.start(owner, title)` returns a handle. Dedupe is the
  owner's job: the update keeps its `Option<TaskHandle>` in `Inner`. It needs
  the handle anyway to re-push the current state on "Show progress".
- **Report:** `handle.report(|state| ...)` changes the state under the lock,
  clones it, drops the lock, then calls `core.apply(Event::Task(state))`, as
  `pump` does for views. The core link is a `Weak<Core>`, set once Core exists,
  as `ViewRunner` does, because `Services` is built before Core.
- **Cancel:** the owner's "cancel" action calls `handle.cancel()`, which flips
  a `tokio::sync::watch`. The work `select!`s on it. tokio's `sync` feature is
  already on. `tokio-util`'s `CancellationToken` would also do, but it is a new
  dependency for one bool.
- **End:** `Done`, `Failed`, and `Cancelled` drop the entry after the report.
  `NeedsInput` stays, and is not "running".
- **Order:** the `Vec` is oldest first, so "the oldest running task" is the
  first `Running` entry.

### The progress view is a protocol view built from the state

```rust
View::Progress(ProgressView {
    title, subtitle, value: Option<f32>, detail,
    steps: Vec<Step { title, status: Status }>, actions,
})
```

`Status` is the accessory enum from `progress-atoms`.
`tasks::progress_view(&TaskState, actions) -> ViewTree` builds it. The update
pushes it through its `InvocationContext`, throttled by `progress_step` as
today, on every report that changes a step or status. So the view is one more
protocol view on the normal stack, and a third-party extension could emit
one.

This extends the versioned view protocol. It stays at version 2: the change is
additive, and every existing tree stays valid.

Rejected:

- **The reducer draws the progress view straight from `Event::Task`.** That
  needs a screen kind outside the view stack, and Escape, back, panel and
  bindings would all need a second path.
- **A task id inside the protocol view.** That leaks Core ids into the
  protocol. The reducer uses the view's owner instead (below).

The reducer gets `Mode::Progress`, which follows `Mode::Detail`:

- the surface takes focus;
- the bindings are Escape, Enter (primary action), and the panel chord;
- the panel works as on a detail.

### `ActionOutcome::Back`

"Continue in background" and "Install later" leave the view and keep the work.
A new `ActionOutcome::Back` is handled in `action_finished` like Escape: pop
one view and abandon the invocation. That is right here, because the task
holds its own handle and not the invocation. Rejected: an Enter rule for
progress views in the reducer, since "Install later" needs the same pop from
the panel.

### The reducer: tasks, background toast, result toast

The reducer keeps the last `TaskState` per id from `Event::Task`. "That task's
progress view" means the top view is a progress view whose owner is the task's
`owner`.

**Background toast:**
- The busy input becomes `working`, then top view loading, then the oldest
  `Running` task while its progress view is not on top. The busy-mark timing
  applies as is.
- `ViewScreen.busy`, which drives the SearchBar bar, is computed at snapshot
  time: marks shown, unless they show a task (no `working`, top view not
  loading, a running task). So a task never draws a loading bar, and a bar
  that lingers its 400 ms after loading ends stays.
- The busy toast for a task source carries the task's Cancel as a toast action.

**Results:**
- When `Event::Task` brings `Done`, `NeedsInput`, or `Failed`:
  - **Launcher hidden:** hold the result as `pending_result`. A failure also
    goes to the HUD at once, as today's hidden failures do.
  - **Launcher shown, with the task's own view on top:** nothing more; the view
    shows it.
  - **Launcher shown otherwise:** show a toast at once.
- On `Activated`, `pending_result` becomes a toast, unless the top view is the
  task's own progress view, which already shows the result after a held hide.
- A failure toast carries Retry when the task names one.

### Toast actions

```rust
pub struct Toast { style, text, action: Option<ToastAction> }
pub struct ToastAction { title: String, shortcut: Shortcut, request: ActionRequest }
```

There is one path for both Retry and Cancel: an `ActionRequest` to the task's
owner, with the action id the task named. No `ToastRun` enum, and no
`Effect::CancelTask`.

**Keys:**
- While a toast with an action shows, its chord is in `bindings`.
- In `intent`, the toast's chord is checked before `clear_failure`. A match
  emits `Effect::RunAction`, clears the toast, and the key does nothing else.
- Toast chords are not bound while the action panel is open: the panel owns
  the keyboard.

**ActionBar:** it draws the label and a `Kbd` after the text. The accessible
description gets the label and chord.

**Chords:**
- **Cancel:** `{ key: ".", modifiers: [Cmd] }`, which is Cmd on macOS and Ctrl
  on Windows. On macOS winit delivers Cmd+. through `cancelOperation:` as a
  press only (winit `view.rs`), hence the macOS hand check.
- **Retry:** `{ key: "r", modifiers: [Cmd] }`. That is Ctrl+R on Windows,
  which is also "Show in Explorer" for an app at root
  (`applications/mod.rs`). The toast wins while it shows. Once it is gone,
  Ctrl+R reveals again.

### The update on a task

```
Install update --> tasks.start("dango.updates", "Dango 0.5.0")
   step "Download" working, progress = received / total   (throttled)
   Ok(bytes)  --> "Download" and "Verify signature" done, "Restart" needs input,
                  detail "Ready to restart"; bytes and the Update kept in Inner
   Err(Download)  --> "Download" failed, Failed { sentence }, retry "install"
   Err(Signature) --> "Verify signature" failed, Failed { sentence }, retry "install"
   cancel --> stop awaiting, drop the join handle, Cancelled, Back
   Restart now --> source.install(kept)   (exits on Windows, restarts on macOS)
```

- **No `on_finished` API.** Verifying takes milliseconds, so a separate
  "Verify signature" working state would barely show. The error kind already
  tells the two failures apart.
- **No check runs while a download runs or waits ready.** `PackagerSource.found`
  is overwritten by every check, so a ready 0.5.0 could otherwise install a
  newer `Update`. `Inner::check` answers the version it already has while a
  task exists, which covers the 6-hourly check and "Check for Updates" alike,
  and needs no change to the source's API.
- **`installing` and `shown` go.** `Inner` holds `task: Option<TaskHandle>` and
  `ready: Option<Vec<u8>>`.
- **Cancel can't stop the blocking download thread:** the updater has no
  abort. The thread finishes its transfer, and its bytes are dropped.
- **"Check for Updates"** offers "Install update", from the same
  `install_action()`.

ActionBar per status in the progress view:

| Status | Enter | Panel |
|---|---|---|
| Running | "Continue in background" (`Back`) | "Cancel" ⌘. |
| Needs input | "Restart now" | "Install later" (`Back`) |
| Failed | "Retry" | - |

### UI parts

| Surface | Design part | Where |
|---|---|---|
| Progress view | "ProgressView view" | `ui/views/progress-view.slint` (new): title + muted subtitle, `Progress { kind: bar }` 4 px, muted detail, step rows with an unannounced `Dot` |
| Toast action | "Toast · failure Retry ⌘R" | `ui/shell/action-bar.slint`: label + `Kbd` after the text |
| Background toast | "Background task · Toast in ActionBar" | existing busy toast with spinner |

No existing view fits the progress view: a detail renders markdown and can't
hold a bar or status dots. A new `ProgressData` global feeds it, with one
writer, `src/slint_shell/render/progress.rs`. A progress view takes the
launcher's maximum height, as a detail does (`render/height.rs`). Each step
row's accessible label is "<step>, <status word>".

### Copy

Written with the ux-writing skill and checked against `interface-copy`. The
waiting state has one name, "ready to restart".

| Where | String |
|---|---|
| Update item, idle, and "Check for Updates" result | "Install update" |
| Update item, downloading | "Show progress" |
| Update item, ready (title) | "Dango 0.5.0 is ready to restart" |
| Update item and progress view, ready | "Restart now" |
| Progress view, ready, panel | "Install later" |
| Progress view, running, Enter | "Continue in background" |
| Any running task | "Cancel" |
| Failure toast action | "Retry" |
| Progress view title | "Dango 0.5.0" |
| Progress view subtitle | "You have 0.4.2" |
| Detail, downloading | "18.4 MB of 29.6 MB", or "18.4 MB" without a total |
| Detail, ready | "Ready to restart" |
| Steps | "Download", "Verify signature", "Restart" |
| Background toast | "Downloading Dango 0.5.0 · 42%", or "Downloading Dango 0.5.0" |
| Result toast, ready | "Dango 0.5.0 is ready to restart" |
| Step accessible label | "<step>, <status word>", for example "Download, working" |

The failure sentences stay as they are today.

## Risks / Trade-offs

- **A cancelled download keeps transferring** in its blocking thread until it
  ends, because the updater has no abort. Bandwidth only: the bytes are
  dropped and nothing installs.
- **About 30 MB stays in memory** while an update waits as ready. "Install
  later" keeps it until Dango quits.
- **Ctrl+R overlap.** "Show in Explorer" is blocked while a failure toast with
  Retry shows.
- **The update no longer restarts on its own.** A user who confirms and walks
  away comes back to "Ready to restart". That is the point of the change.
