# Proposal

## Why

Design system v2 gives long jobs one model: a task that outlives its view,
watched through a ProgressView, reported in the ActionBar when the user looks
elsewhere, and cancellable. Dango's only long job, the app update, does all of
this by hand. Its progress is markdown text, it restarts without asking, it
can't be cancelled, and a failure while hidden goes only to the log. This is
change 3 of the v2 stack, on top of `progress-atoms`, whose progress bar and
status dots it uses.

## What Changes

- **Task registry in Core:** one place that holds every long job's state: its
  title, status, progress, steps, and outcome. Work reports through a handle.
  The reducer sees tasks as events and renders them. The app update is the
  first user.
- **ProgressView:** a new view kind in the view protocol. It shows one task:
  a title and subtitle, a progress bar, a detail line, and named steps, each
  with a status dot. Going back or hiding the launcher leaves the task running.
- **Background toast:** while a task runs and the user is anywhere other than
  its ProgressView, the ActionBar shows a busy toast with the task and its
  progress. A task that ends while the launcher is hidden reports its result
  as a toast on the next open.
- **Cancel:** every running task offers Cancel (Ctrl+. on Windows, Cmd+. on
  macOS) in its ProgressView and in its background toast.
- **Toast action:** a toast can carry one action, shown after its text with
  its chord. Pressing the chord runs it. The update's failure toast gets
  Retry (Ctrl+R / Cmd+R).
- **App update flow:**
  - **BREAKING (behaviour):** the update stops at "Ready to restart" once the
    download is verified. It no longer installs and restarts on its own.
  - Enter runs "Restart now". "Install later" leaves it, and the update item
    in root search then offers "Restart now".
  - The steps are check, download, verify, and install.

## Non-goals

- No other feature moves onto the registry. App indexing and AI commands are
  the adopt-components change.
- No download speed or time left: the updater reports bytes only, and the
  design's rule is not to make numbers up.
- No task persistence across restarts.
- No toast actions beyond the update's Retry and the task's Cancel.
- No multi-task list view. Each task is reached through its own command.
- The protocol stays at version 2: the progress view is additive.

## Platforms

Windows and macOS. The registry, the ProgressView, toasts, and the reducer are
platform-neutral. The install step already differs per platform: an NSIS
installer on Windows, an in-place bundle swap on macOS. It stays as it is, and
only its timing moves behind "Restart now". Cancel uses Ctrl+. on Windows and
Cmd+. on macOS.

## Capabilities

### New Capabilities

- `background-tasks`: long jobs that outlive their view. Covers the progress
  view of one task, the background toast, the result toast on the next open,
  and cancelling.

### Modified Capabilities

- `view-protocol`: a fifth view kind, the progress view.
- `design-system`:
  - The shell holds a progress view.
  - The progress view has its own layout.
  - A toast can carry one action with its chord.
- `app-updates`:
  - The install flow waits at "Ready to restart".
  - The update can be cancelled and retried.
  - A failure while hidden shows as a toast on the next open.

## Impact

- New `src/tasks.rs`: the registry, task handles, and cancellation.
- `src/protocol.rs`: `View::Progress(ProgressView)` with steps.
- `src/ui/`: task events, the background and result toasts, toast actions and
  their bindings, and the progress view in the snapshot.
- `src/app/mod.rs`: the registry in `Services`, and task events forwarded to
  the reducer.
- New `ui/views/progress-view.slint`. `ui/shell/action-bar.slint` gets the
  toast action. New renderer `src/slint_shell/render/progress.rs` and a
  `ProgressData` global.
- `src/extensions/updates/`: the install flow on a task. The source's
  download and install split at "Ready to restart".
- Tests in `src/ui/tests.rs`, `src/slint_shell/tests.rs`, `src/app/tests.rs`,
  and `src/extensions/updates/`.
