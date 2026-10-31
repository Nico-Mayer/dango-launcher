## Context

See proposal.md - Why. Today:

- `ExtensionHost` (`src/extension/mod.rs`) registers every command in an
  extension's manifest when it activates the extension, and asks the
  `EnabledStore` only per extension. `FileConfig` (`src/config/store.rs`) is
  that store.
- Root search candidates come from the registry (`command_candidates`), and
  invocation resolves through it (`resolve_command`, `run_command`), so a
  command missing from the registry is already out of search and reports
  "That command is no longer available" when confirmed.
- `hotkeys::bindings` (`src/hotkeys.rs`) claims a chord for every configured
  command hotkey, enabled or not. `disabled_with_hotkey` logs the N-02 line.
- Only the AI extension reads a command's `enabled`, when it builds its
  manifest from the file (`src/extensions/ai/commands.rs`).

## Goals / Non-Goals

**Goals:**

- One place decides whether a command is offered: the host, from the config.
- Search, hotkeys, and invocation follow from that without their own checks,
  except the hotkey table, which reads the config directly.

**Non-Goals:**

- No new Rust trait method on `Extension`. Extensions do not need to know.
- No change to the manifest or view protocol.

## Decisions

### Filter at registration in the host

The host passes only the enabled commands of an extension to
`Registry::register`. Search and invocation already read the registry, so
both follow with no change.

Alternatives rejected:

- Filter in each extension, as AI does. That is the bug: every extension must
  remember, and today only one does.
- Filter in search and invocation separately. Two checks that can drift, and
  the registry would still list commands that cannot run.

### The enabled store answers per command

`EnabledStore` gets `is_command_enabled(extension_id, command_id) -> bool`,
with a default of `true` so test stores stay as they are. `FileConfig` reads
`extensions.<id>.commands.<id>.enabled`, absent meaning enabled.

Alternative rejected: pass the `Config` into the host. The host is built to
not know the file, and the store is already its view of it.

### Reload re-syncs commands of active extensions

`reload_enabled` also re-registers the commands of every extension that stays
active: unregister its commands, register the enabled ones. It does not call
`deactivate` or `activate`, so services keep running and root providers stay.
Collisions from the re-register go into the `LoadReport` as today. Config
reload already runs `reload_enabled` before rebuilding candidates and
hotkeys, so the order is right.

Alternative rejected: deactivate and reactivate an extension whose command set
changed. It would restart services (clipboard watcher, snippet expansion) for
a change that does not concern them.

### The hotkey table skips disabled commands

`hotkeys::bindings` does not claim a chord for a command with
`enabled: false`. The chord is not grabbed from the OS, so another command or
app can use it, and no conflict is reported for it. `disabled_with_hotkey`
stays as is and is now accurate.

Alternative rejected: register the chord and fail at the press with
"unavailable". It holds a system-wide chord for nothing and reports a conflict
that is not real.

### The AI extension keeps its own check

AI builds its manifest from the file, and skipping a disabled command there
also skips merging its prompt, which could report a problem for a command the
user turned off. The host check is then a no-op for AI. Keeping both is
harmless.

## Risks / Trade-offs

- [The hotkey table reads `enabled` from the config, the host reads it through
  the store; two readers of one value] → Both read the same field of the same
  `Config` after reload; a test covers a disabled command in both.
- [An extension whose root provider lists its own commands would still show a
  disabled one] → No built-in does this today; root providers return items,
  not commands. Out of scope (proposal Non-goals).
- [A command already running when it is disabled] → It finishes. Disabling
  only affects new invocations, as for a disabled extension.

## Migration Plan

None. Alpha, no users. A config that already sets `enabled: false` on a
non-AI command starts being honoured.
