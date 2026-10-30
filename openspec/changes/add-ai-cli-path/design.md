## Context

`CliClient` in `src-tauri/src/extensions/ai/cli.rs` hands `spec.command` to
`process-wrap`, which spawns it through `std::process::Command`. That searches
Dango's inherited PATH only, and on Windows finds only `.exe` files for a bare
name. A spawn failure becomes `AiError::CommandMissing`.

The config file sits in `~/.config/dango/` on both platforms, so one file may be
shared between a Mac and a Windows machine. The config already has a
per-platform value, `PerPlatform` in `src-tauri/src/config/hotkey.rs`, used by
hotkeys and application names. It holds a `String` per side.

## Goals / Non-Goals

**Goals:**

- A bare command name resolves against configured directories, then PATH.
- One shared config file can carry different directories per platform.
- `.cmd` and `.bat` wrappers resolve on Windows.

**Non-Goals:** see the proposal.

## Decisions

### The setting lives on `dango.ai`, not on each provider

`extensions."dango.ai".path`, beside `providers`. Every command provider on a
machine needs the same directories, so per-provider lists would repeat
themselves. If another extension needs a search path later, the key can move to
the top level.

Rejected: a `path` per provider. More precise than anyone needs, and it repeats.

### Shape: a list, or a list per platform

```json
"path": ["~/bin"]
"path": { "macos": ["~/.local/share/mise/shims"], "windows": ["~/AppData/Local/mise/shims"] }
```

This follows the hotkey and app-name pattern: a plain value or a per-platform
object. `PerPlatform` becomes generic over its value (`PerPlatform<T>`, `T`
defaulting to `String` so existing uses are unchanged), and the path setting is
an untagged enum of `Vec<String>` or `PerPlatform<Vec<String>>`. The schema
generator names the `PerPlatform` definition explicitly (`schema.rs`), so the
generic instance needs its own definition name there.

Rejected: a separate `PathList` struct with `macos` and `windows` fields.
It works but duplicates a type the config already has.

### Find the program with the `which` crate, run it by full path

`which::which_in` looks up a name in a given search path and, on Windows,
applies PATHEXT, so `claude` finds `claude.cmd`. The search path is the
configured directories followed by Dango's PATH. A `command` containing a path
separator skips the lookup. When nothing is found, the existing
`CommandMissing` error is returned, so the message does not change.

The resolved full path is what `CommandWrap` spawns. The child's environment
gets `PATH` set to the same joined list (`std::env::join_paths`), so a wrapper
finds its own tools.

Candidates considered:

- `std::process::Command` with `PATH` set on the child. On Windows the standard
  library searches a child PATH that was changed, but still only for `.exe`, so
  an npm `.cmd` wrapper stays unfound.
- `which` (8.x, maintained, used widely). Does the job. Its `which_in` signature
  is confirmed against the vendored source in the first task before use.

### `~` expansion only, through `dirs::home_dir`

`dirs` is already a dependency. Only a leading `~` followed by a separator or
the end is expanded. Environment variables are not expanded (proposal
non-goal).

Rejected: `shellexpand`. A new dependency for one prefix.

### Resolved at run time, not at config load

The directories are read when a command runs, from the `Providers` built by the
config reload. A config edit applies to the next run with no extra wiring, and
nothing runs at startup or on the hotkey path.

## Contracts

No change to the extension manifest or the view protocol. The config gains one
key; the schema and settings reference are regenerated (`config-schema` spec).

## Risks / Trade-offs

- Running a `.cmd` through `std::process::Command` uses `cmd.exe` argument
  rules. Since Rust 1.77 the standard library refuses arguments it cannot quote
  safely for a batch file and returns an error. A prompt passed as a `{prompt}`
  argument with such characters would then fail to start. Prompts on standard
  input, the default, are not affected. Accepted; the error surfaces as a failed
  run, and the user can switch to standard input.
- A wrong directory in the list is silently skipped. The "could not run"
  message still names the command, which points the user at the list.
