## Why

Every setting Dango has, including hotkeys, aliases, and keybinds, lives in
`config.json`, and the only way to reach it today is to remember the path and
open it by hand. A launcher whose settings surface is a file should be able to
open that file from its own prompt, the way Raycast and Wox open their
preferences.

## What Changes

- A new `Open Config File` command in the built-in `system` extension that
  opens `config.json` in whatever application the operating system associates
  with `.json` files. No editor is chosen or configured by Dango.
- The command is found under the words a user reaches for: config, settings,
  preferences, keybinds, hotkeys.
- When no `config.json` exists yet, the command creates a minimal one before
  opening it, so the editor opens a real file rather than failing. This is the
  user asking for the file, so it does not conflict with the rule that Dango
  never creates a config file unasked.
- Like every command, it can be given an alias and a global hotkey in the
  config file under `extensions.dango.system.commands.open-config`.
- Documentation: the README's configuration section names the command.

Milestone: M7 - polish. The file is the settings surface decided in M5; this
is the missing door to it. Platforms: Windows and macOS, both covered.

## Non-goals

- Opening any other file in the config directory: `auth.json`,
  `snippets.json`, `quicklinks.json`, or `dango.log`. One command, one file.
  Others can follow the same shape later if they earn it.
- Choosing or configuring an editor. The request is the operating system's
  default handler for the file type; a `with`-style preference is not added.
- A settings interface, a config preview inside the launcher, or validating
  the file before opening it.
- Revealing the file in Finder or File Explorer.
- Linux.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `system-commands`: the extension gains a fifth command, `Open Config File`,
  with its own requirement covering finding it, creating the file when absent,
  opening it with the platform's default handler, and reporting a failure.

## Impact

- `src-tauri/src/extensions/system/`: one new command, manifest entry, and
  tests.
- `src-tauri/src/config/store.rs`: `FileConfig` gains a way to make sure the
  file exists, writing through its own-write signal so the watcher does not
  treat the creation as an external edit.
- `src-tauri/src/lib.rs`: the system extension is handed a file opener built on
  the already registered `tauri-plugin-opener`, beside the quicklink opener.
- `src/lib/Icon.svelte`: one new named icon for the command.
- `README.md`: one sentence in the configuration section.
- No new dependency, no manifest or protocol version change, no frontend
  capability change.
