## Context

See proposal.md for why. What already exists and shapes this:

- `config::hotkey::parse` turns a chord string into a `(Modifiers, Code)`, and a
  `CommandSettings` in the config already has a `hotkey` field, reserved by
  `add-config-file`. The config watcher and `apply_config_reload` in `lib.rs`
  already re-apply the launcher hotkey and enabled state on an edit.
- The launcher hotkey is one `Shortcut` behind `Arc<Mutex<Shortcut>>`; the
  global-shortcut handler toggles the launcher when the pressed chord matches it.
  Registration and re-registration go through `app.global_shortcut()`.
- `invoke_command(app, qualified_id)` runs a command through the `Invoker` and
  streams `Output`: a `View` is emitted to the webview, `Finished(Success)` hides
  the launcher, a failure emits an error. This is the exact behaviour a hotkey
  should reproduce.
- A command's `InvocationMode` (`View` or `NoView`) lives on its `CommandDecl` in
  the registry, reachable via `host.registry().get(qualified_id)`.
- The Windows text and window paths act on `previous_foreground`, recorded when
  the launcher shows. macOS acts on the frontmost application, which the launcher
  never displaces.

## Goals / Non-Goals

**Goals:**

- Bindings are data in the config file, applied at startup and live, reusing the
  grammar, watcher, and invocation path already built.
- A hotkey press is indistinguishable from picking the command in root search.
- Collisions are never silent.

**Non-Goals:**

- See proposal.md. At design level additionally: no change to the invocation or
  view-protocol contracts, and no new global input mechanism, only more chords
  through the plugin already in use.

## Decisions

### One dispatch table, matched in the handler

The global-shortcut handler already receives the pressed `Shortcut`. Rather than
one launcher chord, it consults a small table: the launcher chord, and a list of
`(Shortcut, qualified_command_id)`. On a press it toggles the launcher for the
launcher chord, or dispatches the matching command, or ignores an unknown chord.
The table is shared behind a lock, the way the launcher chord already is, so a
reload can rebuild it.

A list scanned linearly, not a map, because `Shortcut` need not be hashable and
the binding count is small. The launcher chord is checked first, so it can never
be shadowed by a command binding.

### A hotkey runs a command through the existing invocation path

The streaming logic inside `invoke_command`, run through the `Invoker` and handle
each `Output`, is factored into a shared `run_command(app, qualified_id)` so the
Tauri command and the hotkey dispatch behave identically. A hotkey press calls
it with the bound command's id.

Whether the launcher appears is decided by the command's `InvocationMode`, read
from the registry:

- `NoView`: invoke without showing the launcher. The command runs, and the
  success that would hide the launcher is a no-op because it never showed.
- `View`: show the launcher first, so the warm webview is up to receive the
  pushed view, then invoke. This is the same order as summon-then-pick.

Rejected: always showing the launcher. It would flash the window for every
window-management or snippet hotkey, which is exactly the friction a hotkey
exists to remove.

### A headless command captures the foreground at the moment of the press

A `NoView` command that acts on the user's window, a snippet insertion or a
window move, needs the window that was focused when the chord was pressed. The
launcher normally records that on show, but a headless hotkey never shows it. So
before invoking a `NoView` command, the dispatch records the current foreground
as the previous window, the same value the launcher would have captured. On
Windows that is `GetForegroundWindow`; on macOS the frontmost application is
already the target and nothing is recorded.

This is the one platform-specific line in the change, and it reuses the
`remember_previous_foreground` the text and window managers already read.

### Conflicts are resolved first-wins and reported, at every (re)load

Building the dispatch table is also where conflicts are found:

- Two commands on one chord: the first keeps it, the rest are dropped and named.
- A command on the launcher chord: the launcher keeps it, the command is dropped
  and named.
- An OS registration refusal: caught from `register`, the binding is dropped and
  named, and the others still register.

All three go to the tray status line and the log, reusing the surfacing
`add-config-file` built. First-wins is deterministic and order is the file's, so
the report tells the user exactly which line to change.

Rejected: refusing to start or blocking all bindings when one conflicts. One bad
line should not disable every working hotkey.

### Reload rebuilds every command binding

On a config edit, `apply_config_reload` already re-registers the launcher hotkey
and reloads enabled state. It gains a step: unregister the current command
hotkeys, rebuild the table from the new config, and register the survivors, with
conflicts re-evaluated. Unregistering by the set the app registered, not
`unregister_all`, so the launcher hotkey is left in place and re-registered only
when it actually changed.

## Risks / Trade-offs

- **A chord the OS owns fails at registration, and only there.** The grammar
  cannot know what another app has claimed. → Caught at register time and
  surfaced like any other conflict; the rest keep working.
- **A headless command with a stale foreground if the press races a focus
  change.** → The foreground is read at the instant of the press, the tightest
  the platform allows, the same window the user was just in.
- **A view command shown by a hotkey starts from a shown launcher.** Correct, but
  it means the launcher can appear without the summon chord. → That is the
  intended behaviour for a view command, and matches how picking it in search
  would look.
- **Many bindings mean many registered global shortcuts.** → The plugin handles
  the set; the handler's linear scan is over a handful of entries per press,
  nowhere near a budget.
