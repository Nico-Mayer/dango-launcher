# Proposal

## Why

A Claude subscription, or the subscriptions pi can sign in to, pays for
inference that no API key can use. The vendor's own CLI is already signed in to
that subscription. The generic `cli` kind that ran such a CLI was removed
(`remove-cli-provider`). It opened a console window on Windows, could not find
programs, and loaded the user's whole agent setup. Fixing those problems needed
per-vendor flags, and a generic kind could not carry them. With one provider
kind per CLI, Dango can own those flags.

## What Changes

- Two new provider kinds: `claude` and `pi`. Each one runs that CLI once per
  request, sends the prompt, and uses the printed text as the answer. Output is
  plain text, passed on as it is read.
- Each kind passes its own flags to turn off tools, MCP servers, hooks,
  plugins, skills, context files (CLAUDE.md, AGENTS.md) and saved sessions. It
  also runs the program in an empty folder.
- A provider entry of these kinds takes `model` (optional, the CLI's default
  otherwise) and `path` (optional absolute path to the program).
- The command's thinking level is mapped to the CLI's own effort or thinking
  flag.
- Dango finds the program when a command runs: the configured `path` first,
  then the process PATH, the PATH the user's login shell sets up (macOS), and
  known install folders (mise shims, `~/.local/bin`, npm's global folder,
  WinGet links, scoop, Homebrew). If the program is not found, the command
  fails with a message saying so.
- The program runs without any window. On Windows, a console never opens and
  the launcher keeps focus. Abandoning a request stops the program and
  everything it started.
- These kinds stop after two minutes with no new output. The clock restarts on
  every output. Services keep their 30-second wait for the first output.
- `"kind": "cli"` stays an unknown kind.

## Non-goals

- A generic "run any program" kind. This was removed on purpose.
- Codex. Its shell tool cannot be turned off, and a read-only sandbox still
  lets a prompt read any file. It can come in its own change once its flags are
  checked.
- Extra user arguments (`args`). Most CLIs let a later flag override an
  earlier one, so user args could turn isolation back off.
- Structured streaming from a CLI (`stream-json`, `--json`).
- More CLIs (Gemini, OpenCode and others).
- Detecting installed CLIs ahead of time, offering them in the UI, or
  suggesting provider entries. A missing program is reported only when a
  command runs.
- Signing in. The user signs in to each CLI in a terminal. Dango never reads or
  stores their credentials.
- A long-lived helper process or an agent SDK.

## Platforms

Windows and macOS. Program lookup and hidden spawning differ per platform, so
each gets its own work and hand checks. Windows is built and checked first,
since it is the daily-use machine and where the console bug happened. The macOS
hand checks stay open tasks until a session on the Mac.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ai-providers`: two new provider kinds that run a vendor CLI. The rules for
  finding, isolating, running and stopping it. A model is optional for these
  kinds. A no-new-output limit applies to them. The unknown-kind scenario is
  corrected to match what the config loader does.

## Impact

- `src/config/mod.rs`: `ProviderKind` gains `Claude` and `Pi`;
  `ProviderSettings` gains `path`. Schema and `docs/config.md` regenerated.
- `src/extensions/ai/`: a new process client behind `Completions`, a dispatcher
  that picks the client by kind, per-kind argument building, program lookup,
  new `AiError` variants, and a no-new-output limit in `drive`. Request
  resolution makes the model optional for these kinds and skips the key lookup.
- `src/platform/`: the contained spawn setup and the folders to search, per
  platform.
- `src/app/`: wiring of the dispatcher in place of the single `genai` client.
- `Cargo.toml`: `process-wrap` and `which` come back. `windows` moves from
  0.61 to 0.62, the version process-wrap's `CreationFlags` type comes from.
- `AGENTS.md`: platform traps for GUI-app PATH and hidden console spawning.
