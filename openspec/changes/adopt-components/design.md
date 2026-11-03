# Design

## Context

- **Root search** builds `ResultItem`s in `Core::result_item` from
  `Candidate`s. Neither carries accessories, and root rows render a title,
  subtitle, and icon only (`render/list.rs` `result_row`).
- **Command hotkeys** live in the config file as strings ("ctrl+alt+v", or one
  per platform) and are registered by `hotkeys::bindings`. Nothing formats
  them for display. `keys::shortcut_label` formats protocol `Shortcut`s, but
  it shows Ctrl as ⌘ on macOS. That is right for action shortcuts, which use
  "the command key", and wrong for a global hotkey.
- **AI runs** (`extensions/ai/extension.rs`) push a streaming detail view
  through their `InvocationContext` and stop when `ctx.is_cancelled()`, which
  any newer invocation, Escape, or a reset makes true. `deliver` then shows,
  pastes, or copies.
- **Background tasks** (`task-registry`) give a busy toast with Cancel and
  result toasts. Retry and Cancel are actions of the task's owner. "The task's
  own view" is checked as a *progress view* of the owner on top.

## Decisions

### Root accessories

- `Candidate.accessories: Vec<Accessory>` (protocol type, default empty) is
  copied into `ResultItem.accessories`. `result_row` draws up to three, through
  the same `accessory_data` as list rows. This is internal plumbing only, with
  no manifest or view-protocol change.
- **Permissions:** each warning candidate gets `Accessory::Tag { text: "Needs
  permission", role: Danger }`.
- **Hotkeys:**
  - `ResultItem.hotkey: Option<String>` is filled in `Core::result_item` for
    `Source::Command`, from the config table's command hotkey, resolved for
    this platform.
  - The renderer draws it as a key chord with a new `keys::hotkey_label(&str)`:
    "⌃⌥⇧⌘" order plus the key on macOS, and "Ctrl+Alt+Shift+Win+V" on Windows,
    matching the names `config::hotkey` parses.
  - An unparsable string shows nothing (the config check already reports it).
  - The chord goes after any other accessories, as a fourth candidate cut by
    the three-accessory limit only when an item declares three.

Rejected: converting the hotkey into a protocol `Shortcut` and reusing
`Accessory::Kbd`. It would print Ctrl as ⌘ on macOS, and global hotkeys may use
keys the protocol's shortcut rules refuse.

### AI runs as tasks

- **Start:** a run starts a task (owner `dango.ai`, item id = command id,
  title = command title, line "Running <title>…", cancel action `cancel`).
- **Keeping it alive:** the run's stop condition becomes the task's cancel
  signal instead of `ctx.is_cancelled()`. Views are still pushed through the
  context, and a superseded context just drops them, so leaving the view loses
  nothing.
- **One at a time:** `Inner` keeps the running task and its last text.
  - Running the same command again pushes the last streaming text through the
    new context and adopts it as the view.
  - Running a different command cancels the old task first.
- **End:**
  - In view, `deliver` runs as today, and the task ends `Done` without a toast,
    because the owner's view is on top.
  - Away from its view, with output View or Paste: the answer is kept, and the
    task ends `Done { "<title> answer is ready" }` with a show action. Nothing
    is pasted.
  - Away, with output Copy: the answer is copied, and the task ends
    `Done { "Copied to clipboard" }`.
  - Failure: `Failed { sentence }`. The HUD and toast rules come from
    `task-registry`.
  - A task's `NeedsInput` isn't used.
- **`TaskState.show: Option<String>`:** the owner's action id. The reducer
  attaches it to a success result toast as "Show answer" with Cmd+Enter
  (`{ key: "Enter", modifiers: [Cmd] }`).
- **"In view":** "Is its view on top?" widens from "a progress view of the
  owner" to "any view the owner pushed". So an AI detail on top suppresses the
  result toast, and the busy toast's task source skips it. It is the same
  `shows_progress_of` check without the `View::Progress` match. The update's
  behaviour doesn't change, because its own views are progress views.
- **`show` action:** it pushes the kept answer as the finished detail view (the
  same `detail_tree(answer, false, true)`), through a fresh runner context.

Alternatives rejected:

- **A progress view for AI runs.** The answer is markdown that streams, and a
  progress view has no body.
- **Pasting when a background run finishes.** The user is somewhere else by
  then, and the text would land in whatever has focus.

### Copy

Written with the ux-writing skill and checked against `interface-copy`.

| Where | String |
|---|---|
| Permission warning tag | "Needs permission" |
| AI busy toast | "Running <title>…" (the existing running toast's form) |
| AI result toast | "<title> answer is ready" |
| Toast action | "Show answer" |
| Copy finished away | "Copied to clipboard" (existing) |
| AI view, running, panel | "Cancel" |

### Contracts

There is no view-protocol or manifest change. `Candidate`, `ResultItem`, and
`TaskState` are internal.

### Stacking

This builds on `task-registry` (tasks and toast actions) and `inline-answer`
(the `ResultItem.answer` field). Archive it after both.

## Risks / Trade-offs

- **An AI run now keeps costing tokens after the user leaves.** That is the
  point. Cancel stops it, and starting another AI command replaces it.
- **A background answer toast lasts about two seconds** like any success toast.
  The answer stays kept until the next run, so "Show answer" works as long as
  the toast shows. After that, running the command again starts a new run.
- **Ctrl+Enter on Windows** is also the form submit chord. The toast only shows
  outside forms that matter here, and the toast chord wins while it shows,
  per `task-registry`.
