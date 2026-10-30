## Why

A command provider named by a bare program name (`claude`) fails with "Dango
couldn't run claude" when the program is on the user's shell PATH but not on the
PATH Dango inherits. Dango starts from the tray, autostart, or Finder, so it gets
the saved system PATH, not the one a shell profile builds. Tools installed
through version managers such as mise live only on the shell's PATH.

## What Changes

- `dango.ai` gains a `path` setting: a list of directories Dango searches for a
  command provider's program before its own PATH. The setting is either one list
  for both platforms or a list per platform (`macos`, `windows`), so one shared
  config file can hold the right directories for each machine.
- A leading `~` in an entry is the user's home directory.
- The same directories go before PATH in the environment of the program Dango
  runs, so a wrapper script can find the tools it calls in turn.
- The program is looked up the way a shell does on each platform, so on
  Windows a `.cmd` or `.bat` wrapper (as npm installs) is found by its bare
  name.

## Non-goals

- Reading the PATH of the user's login shell, or running a shell profile to
  find it. Considered and deferred: it adds a startup-adjacent process per
  platform for a problem one setting solves.
- A Dango-wide PATH setting for anything other than AI command providers.
- Expanding environment variables (`%LOCALAPPDATA%`, `$HOME`) in entries.
- Changing how a provider whose `command` is already an absolute path is run.

## Platforms

Windows and macOS. The setting is the same on both; the per-platform form
exists because the directories differ.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ai-providers`: a command provider is found in configured directories before
  Dango's own PATH, and those directories reach the program's environment.

## Impact

- `src-tauri/src/config/mod.rs`: the `path` setting on the extension entry, and
  the per-platform value type generalised from a string to any value.
- `src-tauri/src/extensions/ai/cli.rs` and `commands.rs`: resolve the program
  against the configured directories and pass the extended PATH to the child.
- `src-tauri/src/config/schema.rs`: the generated schema and settings reference
  pick up the new key.
- New dependency: the `which` crate.
