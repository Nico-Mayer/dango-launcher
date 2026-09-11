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

### Running-application icons reuse the platform extraction, not the extension

Icon extraction moves to a shared platform helper that both `AppIndexer` and
`SystemControl` call. No extension-to-extension dependency: that would make
disabling `applications` silently break `system`.

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
- **Deviating from the roadmap twice** (quit-app as a view command, empty trash
  as a view command). → Both are recorded above with reasons. The roadmap's
  milestone intent, exercising contribution types `applications` did not, is
  still met through commands and the view stack.

## Open Questions

- Whether sleep should offer a separate "display sleep" command. Deferred: it
  changes no spec, no approach, and no task here, and is one more `CommandDecl`
  if the author wants it.
