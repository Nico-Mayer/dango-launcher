# Tasks

## 1. Config

- [x] 1.1 Add `Claude` and `Pi` to `ProviderKind` and `path` to `ProviderSettings`; verify a config test round-trips a `claude` entry with `path`, and that `"kind": "cli"` and `"kind": "codex"` are config errors naming the kind (`cargo test config`)
- [x] 1.2 Regenerate the schema and `docs/config.md` (`env DANGO_UPDATE_DOCS=1 cargo test the_settings_reference_is_current`); verify `the_settings_reference_is_current` passes and the `kind` row lists all five kinds

## 2. Request resolution and limits

- [x] 2.1 Add `ProviderKind::is_agent()` and `Request::path`; make `resolve` allow no model for agent kinds; verify tests in `commands.rs` for "agent program with no model" and that a service kind with no model still fails with "Set a model for {provider} in your config file"
- [x] 2.2 Replace the extension's `!is_local()` key check with `Request::needs_key()`; verify a test in `extension.rs` that a `claude` provider with no `auth.json` reaches the client with no key
- [x] 2.3 Change `drive` to take a `Limit` (`FirstOutput`, `Quiet`) and add `limit_for(&Request)`; verify a unit test that `limit_for` gives `FirstOutput(30 s)` for `ollama` and `Quiet(120 s)` for `claude`, and `stream.rs` tests with a hand-fed channel and millisecond limits: `Quiet` fails after a silence, and does not fail while fragments keep arriving past the limit
- [ ] 2.4 Move trimming of the answer into `extension.rs::deliver`; verify the existing empty-answer test still gives "The model returned no text" and a whitespace-only answer does too

## 3. Argument tables

- [ ] 3.1 Add the per-kind argument builders for `claude` and `pi` from the design table (model, thinking); verify unit tests for each kind and each thinking level
- [ ] 3.2 Check by hand, once per CLI with a throwaway prompt, that `claude -p --safe-mode --tools "" --output-format text --no-session-persistence` and `pi -p --no-tools --no-context-files --no-session` each read the prompt from stdin and answer with the existing sign-in; record the result in this task, and change the builder and design if a flag is refused
  - claude: the author confirmed `-p --safe-mode` answers with the existing sign-in. Still to check: the prompt on stdin, together with the other fixed flags.
  - pi: not checked yet.

## 4. Program lookup

- [ ] 4.1 Add the platform-neutral search: configured absolute path, else process PATH + `platform::agent_search_dirs()`, deduplicated, through `which::which_in`; verify unit tests with temp folders: configured path wins, a relative or missing configured path gives `PathMissing` with "Couldn't find {path}. Check the path in your config file.", order is respected, a program added after a miss is found on the next call
- [ ] 4.2 Add the platform-neutral `PATH=` parser for `env` output; verify unit tests on sample output from zsh and fish, including a missing `PATH=` line
- [ ] 4.3 Windows: `agent_search_dirs()` in `platform/windows` with the known dirs from the design (mise shims, `.local\bin`, `%APPDATA%\npm`, WinGet links, scoop shims); verify a Windows-only test that a `claude.cmd` in a temp folder is found by bare name
- [ ] 4.4 macOS: `agent_search_dirs()` in `platform/macos` running `$SHELL -l -c env` in its own process group with a 10 s kill, cached with one re-probe on a miss, plus the known dirs; leave its test run as an open hand check for the Mac session (8.4)

## 5. Process client

- [ ] 5.1 Bump `windows` to 0.62 and fix any API changes in `platform/windows`; verify `mise run check` and `cargo test` pass
- [ ] 5.2 Bring back `process-wrap` and `which`; add `platform::contained_command(program)` (Windows `JobObject` plus `CreationFlags(CREATE_NO_WINDOW)`, macOS `ProcessGroup::leader()`); verify a Windows unit test that the returned `CommandWrap` `has_wrap::<JobObject>()` and that `get_wrap::<CreationFlags>()` contains `CREATE_NO_WINDOW`
- [ ] 5.3 Add `AgentClient`: `stream()` returns `Chunks` at once, the worker does lookup, spawn in `<cache dir>/agent-workspace` with child `PATH` set to the search list, prompt on stdin, stdout passed on as read; verify a test with a script program (`.cmd` on Windows, `sh` on macOS) that echoes stdin back, and a test that `stream()` returns before a script that sleeps before reading stdin has started
- [ ] 5.4 Exit handling: poll `try_wait` after stdout closes, on non-zero exit log stdout and stderr and send `ProgramFailed`; verify tests: stderr text is in the log and not in the answer, a failing script that prints to stdout gives "{provider} stopped with an error. See dango.log for the reason." with that stdout in the log
- [ ] 5.5 Bring back `Chunks::with_cleanup` and call `start_kill` on drop without holding a lock across any wait; verify two tests: a script with a sleeping child that keeps stdout open, and a script that starts a child with output sent to `nul` or `/dev/null` and then exits; in both, dropping `Chunks` returns quickly and the child is gone
- [ ] 5.6 Add `AiError::ProgramMissing` with "Couldn't find {program}. Install it or set its path in your config file."; verify a test that a missing program fails with that message and nothing is spawned

## 6. Wiring

- [ ] 6.1 Add `Clients` dispatching by kind and hold it in `src/app/` where `GenAiClient` was; verify an `app` test with a `claude` provider reaches the agent side while an `ollama` one reaches the service side
- [ ] 6.2 Add the platform traps to `AGENTS.md`: tray-app PATH lacks shell-only folders (`mise activate`) on both platforms, and `CREATE_NO_WINDOW` must go through process-wrap's `CreationFlags` wrapper, not the inner command; verify `mise run check` passes

## 7. Windows hand checks

- [ ] 7.1 Release build with `DANGO_CONFIG_DIR` set to a scratch dir holding a `claude` provider, started through `explorer.exe` so it gets the saved PATH, not the shell's: run Summarize on a selection; verify with `mise run shot` that no console window appeared, and that the launcher stayed foreground and showed the answer
- [ ] 7.2 Same build: verify in `dango.log` or a `dango-ui` snapshot that `claude` was found through `%LOCALAPPDATA%\mise\shims` with no path set; then set a wrong `path` and verify the "Couldn't find {path}" message
- [ ] 7.3 Start a long prompt, press Escape, verify with `Get-Process claude, node -ErrorAction SilentlyContinue` that no agent process is left
- [ ] 7.4 With a hook, an MCP server, and a rule in the global CLAUDE.md set up in the author's Claude Code config, run a command; verify the hook did not run (its side effect is absent), no MCP server process started, the CLAUDE.md rule had no effect on the answer, no new session appears in `claude --resume`, and the sign-in still answered
- [ ] 7.5 Run one command with a `pi` provider; verify it answers, ignores a rule in the global AGENTS.md, and leaves no session behind

## 8. macOS hand checks

- [ ] 8.1 Bundle opened from Finder with a `claude` provider, `claude` reachable only through a PATH line in the fish config or mise shims: run Summarize; verify it answers, no window or Dock icon appears, and the launcher keeps focus
- [ ] 8.2 Start a long prompt, press Escape; verify with `pgrep -fl claude` that nothing is left
- [ ] 8.3 Repeat 7.4 on macOS; verify the same results
- [ ] 8.4 Install a CLI while Dango runs, into a folder only the shell profile adds; verify the next command finds it without a restart; then add a `sleep 30` to the profile and verify the launcher stays responsive and Escape still abandons

## 9. Review

- [ ] 9.1 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context; verify the findings are reported to the author
