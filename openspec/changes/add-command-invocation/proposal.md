## Why

M1 built the shape of the command contract but never the path that runs a
command. The registry records commands, the view protocol describes what a
command can put on screen, and the frontend listens on `dango://render`, but
nothing ever invokes a command and nothing ever emits that event. `applications`
did not expose the gap because it contributes root items, not commands.

Every milestone after this one is made of commands. This change closes the gap
and proves it with the first extension whose whole purpose is commands.

This is the first of two changes in **M2 - builtins** from
`openspec/ROADMAP.md`. The second is `clipboard-history`, which needs the
invocation path this change builds. The roadmap's third item, `calculator`, is
dropped: the author does not use one.

## What Changes

- Root search learns to invoke a command rather than only to run an action on
  an application result. Choosing a command result executes it.
- A command host trait, with the built-in native host as its only implementation,
  so the sandboxed host M8 will add slots in without changing callers.
- No-view commands run and report success or failure, using the outcome type the
  protocol already defines.
- View commands emit a view tree on `dango://render`, which the frontend already
  renders and pushes onto its stack. This is the first thing that ever emits it.
- The action-running command in the backend stops being hardwired to
  `applications` and dispatches by owner, so any extension can own a result.
- A `system` built-in extension ships as the first consumer: lock the screen,
  sleep, empty the trash, and quit a running application. Covered on Windows and
  macOS.

## Capabilities

### New Capabilities

- `command-invocation`: how a command is invoked from root search, which host
  runs it, how a no-view command reports its outcome, how a view command
  delivers its first tree, and what happens when a command fails or is invoked
  while another is running.
- `system-commands`: the built-in extension contributing screen lock, sleep,
  empty trash, and quit application, including the per-platform behaviour of
  each and the running-application list that quit is chosen from.

### Modified Capabilities

- `root-search`: the capability describes how results are gathered and ranked
  but never says what confirming one does. That becomes a requirement here.

`extension-model` is deliberately not modified. Invocation modes, hosts, and
result ownership all describe running a command, so they belong to
`command-invocation`; the failure behaviour invocation needs is already a
requirement there.

## Impact

- **Affected code**: `src-tauri/src/extension/` (host trait, registry lookup
  gains an execute path), `src-tauri/src/lib.rs` (`run_action` generalised, a new
  invoke command, render event emission), `src-tauri/src/extensions/system/`
  (new), `src-tauri/src/platform/` (lock, sleep, and empty trash are per
  platform, behind a trait like every other OS call).
- **Frontend**: no new rendering. The view stack, the action panel, and the
  no-view outcome renderer all exist from M1 and are exercised for the first
  time.
- **Dependencies**: none expected on macOS, which has native calls for every
  system command. Windows may need additional `windows` crate features.
- **Protocol**: no version bump. `ViewTree`, `Action`, and the no-view outcome
  are used as M1 defined them; if they prove wrong, that is a finding worth
  recording rather than a silent reshape.

## Non-goals

- **No `clipboard-history`.** It is the other half of M2 and lands as its own
  change once invocation exists.
- **No `calculator`.** Dropped from the roadmap at the author's request.
- **No command hotkeys.** Binding a command to a global shortcut is M6.
- **No preferences UI.** The typed schema exists from M1; rendering it is M7.
  `system` ships with no preferences so nothing depends on that gap.
- **No third-party or sandboxed host.** The host trait gets exactly one
  implementation. Designing for a second one without having it is how the
  abstraction goes wrong.
- **No paste, no selection capture, no focus restore.** M3 plumbing.
- **No argument or form input to commands.** Forms are in the protocol from M1,
  but no command here needs one, and inventing a consumer to exercise it would
  be backwards.
- **No Linux.** Out of scope for the project.
