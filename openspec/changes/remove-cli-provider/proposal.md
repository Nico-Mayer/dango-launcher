## Why

The `cli` provider kind runs a program such as `claude -p` per request. In
practice it is fragile: on Windows the release build opened a console window
that stole focus, hid the launcher, and cancelled the run; finding the program
needed a PATH setting of its own; and the programs worth driving are agents
that load the user's hooks, MCP servers, and tools for every request. Making it
reliable means knowing each program's flags, which turns a generic provider
into vendor-specific code. Removing it is cheaper than hardening it.

## What Changes

- **BREAKING** The `cli` provider kind is removed. A config entry with
  `"kind": "cli"` is an unknown kind and is surfaced as a configuration problem,
  like any other unknown kind.
- **BREAKING** The `dango.ai.path` setting is removed, along with its schema
  definition and its row in the settings reference.
- The process-running client, its Windows and Unix process containment, and the
  program lookup go away, with the `process-wrap` and `which` dependencies.
- The unfinished `add-ai-cli-path` change is deleted rather than archived, since
  its behaviour never reaches the main specs.
- The author's own config moves its default provider to the local Ollama one
  and drops the `claude` provider and every command override naming it.

## Non-goals

- A replacement way to use a ChatGPT or Claude subscription. If it returns, it
  is a new design, for example a long-lived helper rather than one process per
  request.
- Changes to the Anthropic, OpenAI-compatible, or Ollama providers.
- Changes to how a run ends when the launcher hides (`hold-state-after-dismiss`
  owns that).

## Platforms

Windows and macOS alike. The removed code had per-platform process handling on
both; nothing replaces it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ai-providers`: three provider kinds instead of four, and the requirement for
  a command provider is removed.

## Impact

- `src-tauri/src/extensions/ai/`: `cli.rs` removed; `CliSpec` and the `cli`
  field on `Request` removed; the `Clients` dispatcher collapses to the one
  `genai` client.
- `src-tauri/src/config/`: `ProviderKind::Cli`, the `command` and `args`
  provider fields, `SearchPath`, and the `path` extension field removed;
  `PerPlatform` back to a plain string value; schema and `docs/config.md`
  regenerated.
- `src-tauri/src/lib.rs`: wiring for the CLI client removed.
- `src-tauri/Cargo.toml`: `process-wrap` and `which` removed.
- `AGENTS.md`: the platform trap about tray PATH and `.cmd` lookup removed,
  since nothing spawns programs by name any more.
- `~/.config/dango/config.json` (author's machine, outside the repo).
