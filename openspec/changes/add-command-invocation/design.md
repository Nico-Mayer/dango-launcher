## Context

M1 left three pieces that never met. `Registry` records a `RegisteredCommand`
with a `Host` enum that has one variant and no method. `protocol.rs` defines
`ViewTree`, `EVENT_RENDER`, and a no-view outcome type. `App.svelte` listens on
`dango://render` and maintains a view stack. Nothing in between exists: no
trait a command implements, no code path from a chosen result to running code,
and no emitter for the render event.

`run_action` is the closest thing, and it is hardwired: it reaches for
`Arc<ApplicationsExtension>` out of Tauri state and calls `perform`. That works
because exactly one extension contributes results. It stops working the moment a
second one does.

The search pipeline already solved the shape of the problem this change faces.
Per-keystroke queries cancel and restart, at most one is in flight, and late
output from a superseded query is dropped. Invocation needs the same discipline
for the same reason, and should not invent a second mechanism for it.

## Goals / Non-Goals

**Goals:**

- One path from a chosen result to running code, used by every extension.
- Invocation returns immediately; output arrives over a channel. Streaming in M4
  must not require reopening this.
- `system` exercises both invocation modes and the view stack, so the contract
  is proven by a consumer rather than asserted.

**Non-Goals:**

- No second host implementation. See the proposal's non-goals.
- No change to `ViewTree`, `Action`, or `InvocationMode`. If they turn out
  wrong, that is a recorded finding, not a quiet reshape.
- No sharing of the running-application list with `applications`. They answer
  different questions: what is installed against what is running.

## Decisions

### A command is a trait object, not an enum

`trait Command { fn invoke(&self, ctx: InvocationContext) -> ... }`, with the
extension handing its commands to the host at activation time, exactly as it
already hands over services and a root provider.

Rejected: an enum of built-in commands matched in a central `run` function. It
reads fine with four commands and becomes a god-function by M5, and it gives
third-party commands nowhere to go, since they cannot add an enum variant.

### Output goes to a sink, not a return value

`invoke` receives a sink it can push view trees to and returns nothing
meaningful. A no-view command pushes an outcome instead.

Rejected: `fn invoke(&self) -> Result<ViewTree, Error>`. Simpler today and
wrong by M4, where streaming AI output has to replace the tree repeatedly while
the command is still running. The spec already requires a view command to be
able to deliver a further tree; a return value cannot express that. Since the
sink is the thing M4 will lean on hardest, getting it in now is cheaper than
retrofitting it around a return type every command already uses.

### Supersession reuses the search pipeline's generation counter

One invocation at a time, tagged with a generation. Output from a stale
generation is dropped at the boundary rather than cancelled at the source,
because a command doing OS work cannot always be interrupted safely.

Rejected: letting invocations run concurrently and rendering whichever answers
last. It makes the view stack non-deterministic and gives the user no way to
reason about what they are looking at.

### `run_action` dispatches by owner

The backend keeps a map from extension id to an action handler, populated at
activation. `run_action` reads the owner from the result id and dispatches.

Rejected: keeping the result id opaque and asking every extension in turn
whether it owns it. That is a linear scan with ambiguous ownership when two
extensions accept the same id.

### Quit application is a view command, not a root provider

The roadmap called `system` the milestone's dynamic root provider. It is better
as a view command that pushes a `ListView`: `applications` already proves the
root-provider path, nothing yet proves the view stack, and putting every running
process into root search would collide with the installed-application results
that share their names.

Filtering is `Filtering::Launcher`. The list is small and complete when pushed,
so the command has no reason to re-run per keystroke, and letting the launcher
filter is the cheaper half of the protocol.

### Empty trash confirms through the protocol, not a native dialog

It pushes a `DetailView` naming the item count, with confirm and cancel in the
action panel. This means empty trash is a view command, not the no-view command
the roadmap implies.

Rejected: `tauri-plugin-dialog`. A native modal steals focus from a launcher
whose whole design is about not stealing focus, it looks nothing like the rest
of the surface, and it would be the only confirmation in the product that does
not go through the view stack.

### An action inside a pushed view re-dispatches to the extension

Found while building: `ProtocolView` reports only an action id, and nothing
receives it. A command that pushes a list has no way to hear that one of its
items was chosen, which is most of what quit application and empty trash do.

An action chosen inside a view goes back through the same `run_action` path as a
root result, carrying the item id and the extension that owns the running
invocation. `invoke_command` returns that owner so the frontend knows where to
send it. Nothing is added to the view protocol.

Rejected: keeping the invoked command alive and delivering actions to it, so it
holds state between pushing a view and handling a choice. That is where a
stateful command eventually has to go, and it is probably where M4 lands, but
both commands here are stateless: the chosen application is named by the item
id, and the confirmation needs nothing but the choice. Building an event loop
for a consumer that does not need one would be inventing the requirement. The
re-dispatch path does not block it later.

### Platform calls sit behind one `SystemControl` trait

Lock, sleep, empty trash, list running applications, and quit an application are
one trait in `platform/`, implemented per platform, as the project requires for
all OS access.

Expected implementations, to be confirmed during the spike rather than trusted:

| | macOS | Windows |
|---|---|---|
| Lock | `CGSession` suspend | `LockWorkStation` |
| Sleep | IOKit power request | `SetSuspendState` |
| Empty trash | `NSFileManager` over the trash directory | `SHEmptyRecycleBin` |
| Running apps | `NSWorkspace.runningApplications` | top-level window enumeration |
| Quit | `NSRunningApplication.terminate` | `WM_CLOSE` to the main window |

The Windows column is the risk. It is the unfamiliar side, it cannot be compiled
locally, and CI is the only check, so it gets spiked first the way the shell
enumeration was in M1.

Symbols and features, read off the vendored `windows` 0.61 source rather than
recalled, since Windows code cannot be compiled on the machine it is written on:

| Call | Module | Crate feature |
|---|---|---|
| `LockWorkStation() -> Result<()>` | `Win32::System::Shutdown` | `Win32_System_Shutdown` |
| `SetSuspendState(hibernate, force, wake_disabled) -> bool` | `Win32::System::Power` | `Win32_System_Power` |
| `SHQueryRecycleBinW(root, *mut SHQUERYRBINFO) -> Result<()>` | `Win32::UI::Shell` | `Win32_UI_Shell` |
| `SHEmptyRecycleBinW(hwnd, root, flags) -> Result<()>` | `Win32::UI::Shell` | `Win32_UI_Shell` |

Notes that will otherwise cost an hour each. `SetSuspendState` returns a plain
`bool`, not a `Result`. `SHQUERYRBINFO` needs `cbSize` set before the query and
answers `i64NumItems`. Emptying passes
`SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND`, because the
launcher has already asked and a second native dialog would be the thing this
design rejected. A null root path means every drive.

The macOS lock is a warning for the Windows side: `CGSession`, the documented
route for years, no longer ships on macOS 26 and had to be replaced during the
walkthrough. Verify each of these against a running system rather than trusting
that it still exists.

### Running-application icons reuse the platform extraction, not the extension

Icon extraction moves to a shared platform helper that both `AppIndexer` and
`SystemControl` call. No extension-to-extension dependency: that would make
disabling `applications` silently break `system`.

### Windows lands after macOS, not with it

The task order puts the Windows spike first, which assumes whoever implements
has a Windows machine. Development happens on macOS and the project cannot
cross-compile, so the shared core and macOS are built first and Windows
`SystemControl` returns an unsupported error until the spike runs.

This is a sequencing change, not a scope change. The change is not done until
group 7 is, and until then Windows shows the commands with a clear failure
rather than a silent nothing.

## Risks / Trade-offs

- **The sink shape is a guess about M4.** → It is the one decision here made for
  a milestone that does not exist yet. Mitigated by the fact that the spec
  already demands repeated tree delivery, so it is required today regardless.
- **`system` may be too small to prove the contract.** Four commands, no
  preferences, no streaming. → Accepted. `clipboard-history` is the harder
  consumer and lands next; it is better to find the contract wrong with four
  commands than with none.
- **Quitting by `WM_CLOSE` on Windows is a request, not a guarantee.** An app
  that ignores it simply stays open. → The spec already says a refusal is not a
  failure. Force-quit is deliberately absent; a launcher should not be how you
  lose unsaved work.
- **Empty trash is destructive and is one Enter away in root search.** → The
  confirmation view is required by the spec and its primary action is cancel,
  not confirm, so a reflexive second Enter does nothing.
- **Windows carries a visibly dead extension between groups 6 and 7.** → The
  commands fail with a stated reason rather than appearing to work. Accepted as
  the cost of not having a Windows machine to hand.
- **Deviating from the roadmap twice** (quit-app as a view command, empty trash
  as a view command). → Both are recorded above with reasons. The roadmap's
  milestone intent, exercising contribution types `applications` did not, is
  still met through commands and the view stack.

## Open Questions

- Whether sleep should offer a separate "display sleep" command. Deferred: it
  changes no spec, no approach, and no task here, and is one more `CommandDecl`
  if the author wants it.
