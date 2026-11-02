# Design

## Context

Requests go through one seam: `Completions::stream(Request) -> Chunks`
(`src/extensions/ai/provider.rs`). Today one implementation exists,
`GenAiClient` (`client.rs`), and `src/app/` holds it as
`Arc<dyn Completions>`. `commands.rs::resolve` turns a command and the provider
table into a `Request`. It fails with `NoModel` when no model is named.
`extension.rs` looks a key up unless `Request::is_local()`. Then it runs
`stream::drive` with `FIRST_FRAGMENT_TIMEOUT` (30 s). `drive` uses
`std::time::Instant` and `mpsc::recv_timeout`, so tokio's paused time doesn't
affect it.

The removed `cli.rs` (`git show e913fe8^:src-tauri/src/extensions/ai/cli.rs`)
already solved stdin/stdout piping, UTF-8-safe reading as output arrives
(`pump`), stderr to log, kill on drop through `Chunks::with_cleanup`, and an
empty working folder. Its two failures were the console window and program
lookup (see the proposal). The audit found two more problems in it (below:
blocking before `Chunks` exists, and a lock held across `wait`). Its reading
code can come back. Its spawn and kill code is rewritten.

Facts checked for this design:

- `process-wrap` 10.0.0 (`src/std/job_object.rs`, `creation_flags.rs`):
  `JobObject` reads the flags of a registered `CreationFlags` wrapper and
  keeps them while it adds `CREATE_SUSPENDED`, in either registration order.
  The old bug came from setting `CREATE_NO_WINDOW` directly on the inner
  `Command`, which `JobObject` then overwrote. `CommandWrap::has_wrap` and
  `get_wrap` are public, so a test can check the setup.
- `process-wrap` 10.0.0 depends on `windows` 0.62.2, and `CreationFlags` takes
  that version's `PROCESS_CREATION_FLAGS`. Dango is on `windows` 0.61.
- On the author's Windows machine, the saved user PATH has no mise folder.
  `mise activate` adds it only inside shells. With
  `windows_shim_mode = exe`, the shim is
  `%LOCALAPPDATA%\mise\shims\claude.exe`. An npm global install instead gives
  `claude.cmd`. The shim adds about 0.6 s per run (3.3 s cold) compared with
  the real exe.
- `claude --help` (current install) has `-p`, `--output-format text`,
  `--safe-mode` (turns off CLAUDE.md, skills, installed plugins, hooks, MCP
  servers, custom commands and agents, output styles; auth and model selection
  still work), `--tools ""`, `--no-session-persistence`, `--model`,
  `--effort low|medium|high|xhigh|max`. `--bare` skips OAuth and keychain
  reads, so it cannot use a subscription.
- `pi --help` has `-p`, `--model`, `--thinking off|minimal|low|medium|high|...`,
  `--no-tools`, `--no-extensions`, `--no-mcp`, `--no-skills`,
  `--no-prompt-templates`, `--no-context-files`, `--no-session`.
- In text mode both CLIs are expected to print the answer at the end. Passing
  stdout on as it is read costs nothing and keeps the `ai-commands` streaming
  rule true for any CLI that prints earlier.

## Goals / Non-Goals

**Goals:**

- Each kind's flags, model and effort mapping live in one table per kind, in
  one module.
- The process layer is shared by both kinds and knows nothing about either.
- OS-specific spawn and search code lives in `platform/`, with no `cfg` in
  `extensions/ai`.
- Program lookup and `PATH=` parsing are pure functions, tested on every
  platform.

**Non-Goals:**

- Changing `GenAiClient` or the three service kinds.
- Any UI change. The existing working view and answer view are used as they
  are.

## Decisions

### One provider kind per CLI

`ProviderKind` gains `Claude` and `Pi`. `ProviderSettings` gains
`path: Option<PathBuf>`, ignored for the service kinds.

Alternatives considered:

- **Generic `cli` kind (the removed design).** Isolation, effort and model
  flags then live in each user's config. The user has to know each vendor's
  spelling, and Dango cannot make sure tools are off. This was rejected in
  `remove-cli-provider` for this reason.
- **Generic kind plus shipped preset entries.** Presets that always exist break
  the "exactly one provider becomes the default" rule and the "no AI provider
  is set up" message, or need special cases in both. Presets the user copies in
  are only example config.
- **Per-CLI kinds.** Chosen. The vendor knowledge `remove-cli-provider` did not
  want in a generic launcher is accepted now. It is two small tables, and it is
  the only way the isolation requirement holds.

Dropped from the first draft after the audit:

- **Codex.** It cannot turn off its shell tool. `--sandbox read-only` still
  allows reads, so selected text that says "print `~/.ssh/id_ed25519`" could
  put a key into an answer that gets pasted. Its flags are also unverified here.
- **`args`.** Most option parsers let the last value of a repeated option win,
  so `"args": ["--tools", "default"]` would turn tools back on. No test could
  prove a "can't remove Dango's flags" rule.

### Per-kind argument tables

One function per kind builds the argument list from model and thinking. The
prompt always goes on stdin, so no prompt text is ever a command-line argument.
That avoids Windows argument escaping, which matters when the program is a
`.cmd` wrapper run through `cmd.exe`.

| Kind | Fixed args | Model | Thinking off / low / medium / high |
|---|---|---|---|
| `claude` | `-p --output-format text --safe-mode --tools "" --no-session-persistence` | `--model M` | omitted / `--effort low` / `medium` / `high` |
| `pi` | `-p --no-tools --no-extensions --no-mcp --no-skills --no-prompt-templates --no-context-files --no-session` | `--model M` | `--thinking off` / `low` / `medium` / `high` |

Claude has no "off" effort level, so off leaves the flag out. Pi has one, so
off is passed explicitly, so pi does not fall back to its own default level.

`--safe-mode` replaces the first draft's `--strict-mcp-config
--setting-sources "" --disable-slash-commands`. It covers CLAUDE.md, which those
did not. And an empty `--setting-sources` value is not documented. Whether
`--safe-mode` works with `-p`, keeps the sign-in, and reads the prompt from
stdin is checked by hand (task 3.2, tasks 7.4 and 8.3).

Alternative: `--bare` for claude. Rejected, because it turns off OAuth, which
is the reason this kind exists.

### The process layer

`AgentClient` implements `Completions`. `stream()` creates the channel, starts
a worker thread, and returns `Chunks` at once, so `drive` sees Escape and the
time limit from the first moment. The worker:

1. finds the program (next decision),
2. builds the command with `platform::contained_command(program)`, which
   returns a `process_wrap::std::CommandWrap`. On Windows it has `JobObject`
   and `CreationFlags(CREATE_NO_WINDOW)`, both as wrappers. On macOS it has
   `ProcessGroup::leader()`,
3. sets the working folder to `<cache dir>/agent-workspace`, created empty if
   missing,
4. sets the child's `PATH` to the folder list the lookup used, so wrapper
   scripts find `node` and mise shims find `mise`,
5. spawns it and stores the kill handle where the `Chunks` cleanup can reach
   it, then writes the prompt to stdin and closes stdin,
6. reads stdout as it arrives, decoding UTF-8 across read boundaries, and
   sends each piece as a fragment. A second thread reads stderr,
7. after stdout closes, polls `try_wait` until the program exits. On exit 0
   the answer stands. On any other exit it logs stdout and stderr and sends
   `ProgramFailed`, which replaces what was shown.

**Kill without holding a lock across `wait`.** The old code held the child's
mutex while it called `wait()`. `JobObjectChild::wait` waits for every process
in the job, so a leftover grandchild kept the lock, and the cleanup on Escape
blocked on it. Now the worker locks only to call `try_wait` and never blocks
while it holds the lock. The cleanup locks and calls `start_kill`. Dropping
`Chunks` returns at once in every case.

Trimming moves to one place, `extension.rs::deliver`, for every kind.

`std::process` plus threads, as before, rather than `tokio::process`. The work
is blocking reads, and threads keep tokio's features as they are.

Hand-rolling job objects and `killpg` was rejected before for the same reason
it is now: `process-wrap` does the job.

Alternative: keep `CREATE_NO_WINDOW` on the inner command and drop
`JobObject`. Rejected: then killing the wrapper leaves the agent it started
running.

`windows` moves to 0.62 for the whole crate, instead of a second renamed 0.62
dependency. `Cargo.lock` already holds 0.62.2 for other crates, so this adds
no new version to the build. 0.61.3 stays, pulled by other crates. API changes between 0.61 and 0.62 in
`platform/windows` are fixed in the same commit.

### The no-new-output limit

`drive` takes a `Limit` instead of a duration:

- `Limit::FirstOutput(30 s)` for services, as today,
- `Limit::Quiet(120 s)` for agent kinds: the clock restarts on every fragment.

A pure `limit_for(&Request) -> Limit` picks it, and is unit tested. `drive`'s
quiet behaviour is tested with a hand-fed channel and millisecond limits.

For CLIs that print only at the end, `Quiet` works out as a limit on the whole
run. It is still the right rule: it never cuts off a program that keeps
printing.

### Program lookup

```
path set in config  --> must be absolute and exist, else PathMissing
        |
        v (no path)
search dirs = process PATH + login PATH (macOS) + known dirs   (deduplicated, in order)
        |
which::which_in(program, search dirs, workspace)   (PATHEXT on Windows)
        |
found --> run      missing --> (macOS: probe the login shell again once) --> ProgramMissing
```

- **Windows**: no login-shell PATH. A Dango started at login gets Explorer's
  PATH, which is the saved PATH. The known dirs cover what `mise activate`,
  npm and the Claude installer add. Reading the registry would only help when
  a PATH folder is added during the session, so it is left out.
- **macOS login PATH**: run `$SHELL -l -c env` (falling back to `/bin/zsh`),
  in the workspace folder with stdin closed, inside its own process group. It
  is killed after 10 s. It is not interactive (`-i`), so it can't wait on a
  prompt or print a banner. The result is cached for the process. On a miss it
  runs once more, so a newly added folder is found without a restart. It runs
  on the worker thread, so a slow profile never blocks the launcher.
- **`PATH=` parsing** is a pure function in `extensions/ai`, tested on every
  platform. Only running the shell sits in `platform/macos`.
- **Known dirs**: Windows `%MISE_DATA_DIR%\shims` or
  `%LOCALAPPDATA%\mise\shims`, `%USERPROFILE%\.local\bin`, `%APPDATA%\npm`,
  `%LOCALAPPDATA%\Microsoft\WinGet\Links`, `%USERPROFILE%\scoop\shims`. macOS
  `$MISE_DATA_DIR/shims` or `~/.local/share/mise/shims`, `~/.local/bin`,
  `/opt/homebrew/bin`, `/usr/local/bin`.
- `platform::agent_search_dirs()` returns login PATH + known dirs per
  platform. Ordering, deduplication and the `which_in` call are
  platform-neutral and tested with fake folders.

Crates evaluated:

- `which` 8: finds a program in a given folder list, handles PATHEXT. Used.
- `fix-path-env` (tauri-apps): runs `$SHELL -ilc` and writes the result into
  the process environment with `set_var`, meant to be called at the start of
  `main`. That puts a shell run on the startup path, which AGENTS.md rules out.
  It is also interactive, and `set_var` from a background thread is unsound
  with tokio and Slint threads running. Not used. Dango needs the PATH value,
  not a changed process environment, and a short probe gives that.
- Calling `mise which <tool>`: needs `mise` itself to be found first, and
  covers only mise. The shims folder in the known dirs covers the same case.

### Request and key handling

- `Request` gains `path: Option<PathBuf>`. `ProviderKind::is_agent()` drives
  four things: `resolve` allows no model; the extension skips the key lookup (a
  new `Request::needs_key()` replaces the `!is_local()` check there);
  `limit_for` picks `Quiet`; the dispatcher picks the client.
- `Clients { service: GenAiClient, agent: AgentClient }` implements
  `Completions` and picks by kind. `src/app/` holds it where it held
  `GenAiClient`. Tests keep using `Scripted`.

### Strings

Written with the ux-writing skill, checked against
`openspec/specs/interface-copy`:

- `AiError::ProgramMissing`: "Couldn't find {program}. Install it or set its
  path in your config file." (`{program}` is `claude` or `pi`.)
- `AiError::PathMissing`: "Couldn't find {path}. Check the path in your config
  file."
- `AiError::ProgramFailed`: "{provider} stopped with an error. See dango.log
  for the reason."
- Unchanged and reused: "The model didn't answer in time", "The model returned
  no text".

A signed-out CLI exits with an error. Dango cannot tell that apart from other
failures without reading each vendor's error text, so it gets the
`ProgramFailed` message. Both outputs go to the log, since `claude -p` prints
some errors on stdout.

### Contracts

No change to the extension manifest or the view protocol. The config file
gains two provider kinds and one provider field.

## Risks / Trade-offs

- [A CLI update renames or drops a flag, and the run fails] -> The program
  exits with an error, the user sees `ProgramFailed`, and the log names the
  flag. Fix the table in a commit.
- [`--safe-mode` refuses `-p`, drops the sign-in, or still loads a hook] ->
  Hand checks 7.4 and 8.3 before archive. If so, go back to the separate flags
  (`--strict-mcp-config`, `--setting-sources local`,
  `--disable-slash-commands`) and add the missing ones.
- [Child `PATH` set to the search list hides something the program needs] ->
  The list starts with the process PATH, so it only adds folders.
- [Agent warm-up takes seconds per request, plus 0.6 s through the mise shim]
  -> Accepted. These kinds suit Summarize and Explain. Fast transforms stay on
  Ollama. A configured `path` to the real exe skips the shim.
- [A CLI that prints only at the end and needs more than two minutes is cut
  off] -> Accepted for now. Raise the limit or add structured streaming to that
  kind in a later change.
- [The macOS login-shell probe runs the user's profile, which can be slow or
  print] -> Runs once per process, on the worker thread, killed after 10 s.
  Only the `PATH=` line is read.
- [`windows` 0.62 changes APIs used in `platform/windows`] -> Fixed in the
  bump commit, checked by `mise run check` and `cargo test` before anything
  else in group 5.
