## Why

The heart of **M5 - keys**: binding any command to a global hotkey. The config
foundation already reads and parses hotkeys and reserves the place for them at
`extensions.<id>.commands.<id>.hotkey`; this change makes those bindings real. A
window-management move like left-half, a snippet, or any command becomes a single
keypress from anywhere, which is the reason a launcher user reaches for a tool
like this.

It rides entirely on what earlier changes built: the portable hotkey grammar and
the file watcher from `add-config-file`, the global-shortcut plugin already
registering the launcher hotkey, and the invocation path the launcher already
uses to run a command. So this is wiring those together, plus the one genuinely
new concern, telling the user when two bindings collide.

## What Changes

- Any command may declare a global hotkey in `config.json`. Dango registers each
  one at startup and re-registers on a live edit, next to the launcher hotkey.
- Pressing a command's hotkey invokes it exactly as selecting it in root search
  and pressing Enter: a no-view command runs without showing the launcher, and a
  view command shows the launcher with its pushed view.
- A no-view command that acts on the user's current window, a snippet insertion
  or a window-management move, targets the window that was focused when the hotkey
  was pressed, because the launcher never took focus.
- Conflict detection: two commands bound to the same chord, a command bound to
  the launcher's chord, and a chord the OS refuses to register are all detected
  and surfaced through the tray status line and the log. The first binding to a
  chord wins; the rest are reported rather than silently dropped.

## Capabilities

### New Capabilities

- `command-hotkeys`: how a command is bound to a global hotkey, what pressing one
  does for a view and a no-view command, which window a headless command acts on,
  how bindings reload live, and how conflicts are detected and surfaced.

### Modified Capabilities

None. The launcher hotkey and its configurability are already covered by
`global-activation`; this adds command bindings alongside it without changing
that behaviour.

## Impact

- **Affected code**: `src-tauri/src/lib.rs` (register command hotkeys from the
  config, dispatch on press, extend the config reload to rebuild them, and detect
  and surface conflicts), reusing `config::hotkey`, the global-shortcut plugin,
  and the invocation path already in place. A small shared helper factors the
  command-run streaming out of the existing `invoke_command` so the hotkey
  dispatch and the frontend share it.
- **Config**: the reserved `extensions.<id>.commands.<id>.hotkey` key becomes
  live. No schema change; the JSON Schema and example already document it.
- **Dependencies**: none new.
- **View protocol**: unchanged.

## Non-goals

- **No hotkey recorder UI.** Bindings are edited in `config.json`, the whole
  point of the M5 rework. A settings UI remains out of scope.
- **No hyperkey remap.** Binding `hyper+X` already works because the grammar
  expands `hyper` to the four-modifier chord; making CapsLock emit that chord
  system-wide is the separate `add-hyperkey` change.
- **No automatic conflict resolution.** Conflicts are reported, not silently
  reassigned; the user resolves them by editing the file.
- **No per-application or context-specific hotkeys.** A binding is global, as the
  OS registers it.
- **No Linux.** Out of scope for the project.
