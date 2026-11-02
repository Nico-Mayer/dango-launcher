# Tasks

## 1. Config

- [x] 1.1 Add `Claude` and `Pi` to `ProviderKind` and `path` to `ProviderSettings`; verify a config test round-trips a `claude` entry with `path`, and that `"kind": "cli"` and `"kind": "codex"` are config errors naming the kind (`cargo test config`)
- [x] 1.2 Regenerate the schema and `docs/config.md` (`env DANGO_UPDATE_DOCS=1 cargo test the_settings_reference_is_current`); verify `the_settings_reference_is_current` passes and the `kind` row lists all five kinds

## 2. Request resolution and limits

- [x] 2.1 Add `ProviderKind::is_agent()` and `Request::path`; make `resolve` allow no model for agent kinds; verify tests in `commands.rs` for "agent program with no model" and that a service kind with no model still fails with "Set a model for {provider} in your config file"
- [x] 2.2 Replace the extension's `!is_local()` key check with `Request::needs_key()`; verify a test in `extension.rs` that a `claude` provider with no `auth.json` reaches the client with no key
- [x] 2.3 Change `drive` to take a `Limit` (`FirstOutput`, `Quiet`) and add `limit_for(&Request)`; verify a unit test that `limit_for` gives `FirstOutput(30 s)` for `ollama` and `Quiet(120 s)` for `claude`, and `stream.rs` tests with a hand-fed channel and millisecond limits: `Quiet` fails after a silence, and does not fail while fragments keep arriving past the limit
- [x] 2.4 Move trimming of the answer into `extension.rs::deliver`; verify the existing empty-answer test still gives "The model returned no text" and a whitespace-only answer does too

## 3. Argument tables

- [x] 3.1 Add the per-kind argument builders for `claude` and `pi` from the design table (model, thinking); verify unit tests for each kind and each thinking level
- [x] 3.3 Move each CLI's program name, fixed flags, model flag, effort flag and off level into one table entry per kind, and give `claude` `DISABLE_AUTOUPDATER=1` in its entry's environment; verify the `args` tests still pass and a test that the claude environment holds the variable and pi's is empty
- [ ] 3.2 Check by hand, once per CLI with a throwaway prompt, that `claude -p --safe-mode --tools "" --output-format text --no-session-persistence` and `pi -p --no-tools --no-context-files --no-session` each read the prompt from stdin and answer with the existing sign-in; record the result in this task, and change the builder and design if a flag is refused
  - claude: confirmed. The full fixed flag set, with the prompt on stdin, from an empty folder, answered `pong` with exit 0.
  - pi: no flag refused, and the prompt on stdin started a request. It failed because the `openai-codex` sign-in has expired ("Your session has ended"). The error goes to stderr with exit 1, which is what `ProgramFailed` expects. Deferred: the author has no subscription pi can sign in to, so whether it answers stays open until one is available. Without `--no-extensions`, pi prints warnings about the user's extension packages, so that flag matters.

## 4. Program lookup

- [x] 4.1 Add the platform-neutral search: configured absolute path, else process PATH + `platform::agent_search_dirs()`, deduplicated, through `which::which_in`; verify unit tests with temp folders: configured path wins, a relative or missing configured path gives `PathMissing` with "Couldn't find {path}. Check the path in your config file.", order is respected, a program added after a miss is found on the next call
- [x] 4.2 Add the platform-neutral `PATH=` parser for `env` output; verify unit tests on sample output from zsh and fish, including a missing `PATH=` line
- [x] 4.3 Windows: `agent_search_dirs()` in `platform/windows` with the known dirs from the design (mise shims, `.local\bin`, `%APPDATA%\npm`, WinGet links, scoop shims); verify a Windows-only test that a `claude.cmd` in a temp folder is found by bare name
- [x] 4.4 macOS: `agent_search_dirs()` in `platform/macos` running `$SHELL -l -c env` in its own process group with a 10 s kill, cached with one re-probe on a miss, plus the known dirs; leave its test run as an open hand check for the Mac session (8.4)

## 5. Process client

- [x] 5.1 Bump `windows` to 0.62 and fix any API changes in `platform/windows`; verify `mise run check` and `cargo test` pass
- [x] 5.2 Bring back `process-wrap` and `which`; add `platform::contained_command(program)` (Windows `JobObject` plus `CreationFlags(CREATE_NO_WINDOW)`, macOS `ProcessGroup::leader()`); verify a Windows unit test that the returned `CommandWrap` `has_wrap::<JobObject>()` and that `get_wrap::<CreationFlags>()` contains `CREATE_NO_WINDOW`
- [x] 5.3 Add `AgentClient`: `stream()` returns `Chunks` at once, the worker does lookup, spawn in `<cache dir>/agent-workspace` with child `PATH` set to the search list, prompt on stdin, stdout passed on as read; verify a test with a script program (`.cmd` on Windows, `sh` on macOS) that echoes stdin back, and a test that `stream()` returns before a script that sleeps before reading stdin has started
- [x] 5.4 Exit handling: poll `try_wait` after stdout closes, on non-zero exit log stdout and stderr and send `ProgramFailed`; verify tests: stderr text is in the log and not in the answer, a failing script that prints to stdout gives "{provider} stopped with an error. See dango.log for the reason." with that stdout in the log
- [x] 5.5 Bring back `Chunks::with_cleanup` and call `start_kill` on drop without holding a lock across any wait; verify two tests: a script with a sleeping child that keeps stdout open, and a script that starts a child with output sent to `nul` or `/dev/null` and then exits; in both, dropping `Chunks` returns quickly and the child is gone
- [x] 5.6 Add `AiError::ProgramMissing` with "Couldn't find {program}. Install it or set its path in your config file."; verify a test that a missing program fails with that message and nothing is spawned
- [x] 5.7 Finish a run when the program exits even if a helper it started keeps stdout or stderr open: after the exit, wait a short grace period for the pipes to close, then end with what was printed; verify a Windows test where a batch file starts a helper that inherits the pipes, prints an answer, and exits, and the answer arrives well before the quiet limit
- [x] 5.8 Reap the killed program after an abandoned run, so no zombie process is left on macOS; verify the abandon tests still pass on Windows, and leave the zombie check to the Mac session (8.2)
- [x] 5.9 Add a test that a UTF-8 character split across two reads arrives whole; verify it passes and fails when the hold-back branch is removed
- [x] 5.10 Replace the millisecond budgets in the agent tests with checks of order or of the process being gone; verify `cargo test agent::` passes
- [x] 5.11 macOS: read the login shell's `env` output as bytes and convert lossily, so one variable that isn't valid UTF-8 doesn't lose the PATH; leave its run to the Mac session (8.4)

## 6. Wiring

- [x] 6.1 Add `Clients` dispatching by kind and hold it in `src/app/` where `GenAiClient` was; verify an `app` test with a `claude` provider reaches the agent side while an `ollama` one reaches the service side
- [x] 6.2 Add the platform traps to `AGENTS.md`: tray-app PATH lacks shell-only folders (`mise activate`) on both platforms, and `CREATE_NO_WINDOW` must go through process-wrap's `CreationFlags` wrapper, not the inner command; verify `mise run check` passes
- [x] 6.3 Stop every running agent program when Dango quits from the tray: `Completions::stop_all`, kept by `AgentClient`, forwarded by `Clients`, called from `Core::quit`; verify an agent test that `stop_all` stops a running script's child, and an app test that quitting calls it

## 7. Windows hand checks

- [x] 7.1 Quit the installed Dango. Release build started from PowerShell with `DANGO_CONFIG_DIR` set to a scratch dir holding a `claude` provider, and PATH reset to the saved value first (`$env:Path = [Environment]::GetEnvironmentVariable('Path','User') + ';' + [Environment]::GetEnvironmentVariable('Path','Machine')`), so it gets what a login start gets, not the shell's mise folders. `explorer.exe` would drop `DANGO_CONFIG_DIR`, so the build would run on the real config and move the autostart entry: run Summarize on a selection; verify with `mise run shot` that no console window appeared, and that the launcher stayed foreground and showed the answer
  - Passed 2026-11-02: release build, PATH without any mise folder. Summarize answered in the launcher, the launcher stayed foreground, and no console window showed in the screenshot.
- [x] 7.2 Same build: verify in `dango.log` or a `dango-ui` snapshot that `claude` was found through `%LOCALAPPDATA%\mise\shims` with no path set; then set a wrong `path` and verify the "Couldn't find {path}" message
  - Passed: no path set, and `dango.log` holds a mise warning from claude's stderr, so it ran through the mise shim. With `path` set to `C:/nope/claude.exe` the toast read "Couldn't find C:/nope/claude.exe. Check the path in your config file."
- [x] 7.3 Start a long prompt, press Escape, verify with `Get-Process claude, node -ErrorAction SilentlyContinue` that no agent process is left
  - Passed on the rerun: a test command asking for a 2000-word essay with thinking high. While it ran, Dango's children were claude.exe (twice), mise.exe and conhost.exe. The first check after Escape, within half a second, found none, and the launcher stayed open. The first, unclear run most likely happened while the launcher was held hidden.
- [x] 7.4 With a hook, an MCP server, and a rule in the global CLAUDE.md set up in the author's Claude Code config, run a command; verify the hook did not run (its side effect is absent), no MCP server process started, the CLAUDE.md rule had no effect on the answer, no new session appears in `claude --resume`, and the sign-in still answered
  - Passed: test SessionStart hook, user-scope MCP server, and a CLAUDE.md rule ("end every reply with PINEAPPLE7"). A control run without `--safe-mode` left all three traces. The Dango run left none, and no session folder exists for the workspace. The test items were removed afterwards.
- [ ] 7.5 Deferred (no subscription for pi). Run one command with a `pi` provider; verify it answers, ignores a rule in the global AGENTS.md, and leaves no session behind

## 8. macOS hand checks

- [ ] 8.1 Bundle opened from Finder with a `claude` provider, `claude` reachable only through a PATH line in the fish config or mise shims: run Summarize; verify it answers, no window or Dock icon appears, and the launcher keeps focus
- [ ] 8.2 Start a long prompt, press Escape; verify with `pgrep -fl claude` that nothing is left
- [ ] 8.3 Repeat 7.4 on macOS; verify the same results
- [ ] 8.4 Install a CLI while Dango runs, into a folder only the shell profile adds; verify the next command finds it without a restart; then add a `sleep 30` to the profile and verify the launcher stays responsive and Escape still abandons

## 9. Review

- [x] 9.1 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context; verify the findings are reported to the author
