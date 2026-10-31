## Why

The config schema says a command's `"enabled": false` (at
`extensions.<id>.commands.<id>.enabled`) removes a command the extension ships,
but only the AI extension honours it. Every other built-in command still shows
in root search and runs from its hotkey, while the log line added for N-02
(`<ext>.<cmd>: has a hotkey but is disabled`) says it is disabled. The specs do
not define disabling a single command at all. Audit finding N-04.

## What Changes

- The extension host honours a command's `enabled: false` for every extension,
  built-in or not: the command is left out of the registry, so it is not in
  root search and cannot be invoked.
- A disabled command's configured hotkey is not registered with the OS. The
  existing N-02 log line stays and is now true.
- Turning a command off or on in the file applies on reload, without a restart,
  and without restarting the extension's services.
- The extension as a whole stays the unit for services and root items. A
  disabled command only removes that command.

## Non-goals

- No UI to turn commands on or off. The file is the only way, as today.
- No per-command control over root items providers or services. Disabling a
  command does not hide items an extension's root provider returns.
- No change to the AI extension's own handling of `enabled: false`, beyond it
  no longer being the only one that works.
- No change to the config schema text, which already describes this.

## Platforms

Windows and macOS. The change is in platform-neutral code (extension host,
hotkey table, config reload), so both get it with no platform work.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `extension-model`: the "Enable and disable is the unit of control"
  requirement allows one command to be turned off, and a new requirement
  defines what a disabled command does in search, hotkeys, and invocation.
- `command-hotkeys`: a disabled command's hotkey is not registered and takes
  part in no conflict.

## Impact

- `src/extension/mod.rs`: the host skips disabled commands when it registers
  an extension's commands, and re-syncs them on reload.
- `src/config/store.rs`, `src/extension/mod.rs`: the enabled store answers
  per command as well as per extension.
- `src/hotkeys.rs`: the binding table skips disabled commands.
- `src/app/mod.rs`: config reload re-syncs commands before candidates and
  hotkeys are rebuilt.
