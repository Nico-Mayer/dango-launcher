## Context

The `cli` kind lives in `src-tauri/src/extensions/ai/cli.rs` (`CliClient`),
reached through the `Clients` dispatcher in `extensions/ai/mod.rs`. Config
carries `ProviderKind::Cli`, the provider fields `command` and `args`, and,
since `add-ai-cli-path`, a `dango.ai.path` search list built on a generic
`PerPlatform<T>`.

The trigger was a Windows release run: `process-wrap`'s `JobObject` wrapper
calls `creation_flags(CREATE_SUSPENDED | <CreationFlags wrapper value>)`,
overwriting the `CREATE_NO_WINDOW` set directly on the command
(`process-wrap-10.0.0/src/std/job_object.rs`). A GUI-subsystem Dango has no
console to share, so the child got a visible one, which took focus, hid the
launcher, and cancelled the run. `tauri dev` and `cargo test` run with a
console, so neither showed it.

## Goals / Non-Goals

**Goals:**

- No code path in Dango spawns a model program.
- No config key, schema definition, or doc row left for the removed kind.

**Non-Goals:** see the proposal.

## Decisions

### Remove rather than harden

Alternatives considered:

- **Fix the process layer only.** Wrap `CreationFlags(CREATE_NO_WINDOW)`, move
  spawning behind a platform helper, test for a hidden console. Fixes the
  window, leaves the rest: `claude -p` in text mode prints only at the end, so
  the 30 second first-fragment timeout fails long answers; stdout carries
  errors as answers; each run loads the user's hooks, MCP servers, and tools.
- **A first-class `claude-code` kind.** Dango passes `stream-json` output,
  disables tools, MCP, and setting sources, and parses events. Fixes streaming,
  errors, and isolation, but makes a generic launcher carry one vendor's flag
  set, and `--bare`, the one switch that isolates fully, turns off subscription
  login.
- **Remove.** Chosen. The feature only paid off as a way to spend a
  subscription, and the author's default moves to local Ollama.

### `Clients` collapses into the one client

With one kind of client left, the dispatcher has nothing to choose. The
extension receives the `genai` client directly; `Completions` stays as the seam
tests answer through.

### `PerPlatform` returns to a string value

The generic form existed only for the path list. Back to `Option<String>` per
side with no generic parameter, so hotkeys and app names read as before.

### `add-ai-cli-path` is deleted, not archived

Archiving would merge a requirement into the main spec that this change removes
again. Its commits stay in history.

### Unknown kind, no special message

`"kind": "cli"` fails to parse like any other unknown kind and surfaces as a
configuration problem (existing behaviour of the `ai-providers` spec). A
dedicated "this kind was removed" message would be a compatibility shim, which
the project stage rules out.

## Contracts

No change to the extension manifest or the view protocol. The config file loses
a provider kind, two provider fields, and one extension key.

## Risks / Trade-offs

- Commands the author pointed at `claude` now use a smaller local model, so
  answers may be weaker. Accepted; an OpenRouter or Anthropic provider is one
  config edit away.
- `is_local` loses its second kind; the "local model could not be reached"
  message keeps working for Ollama.
