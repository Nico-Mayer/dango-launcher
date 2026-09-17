# Dango roadmap

A staged path from an empty Tauri scaffold to a Raycast-class launcher on Windows
and macOS. Each milestone is one or more OpenSpec changes. Order reflects
dependencies, not a schedule.

See `openspec/config.yaml` for the architectural spine, platform stance, and the
constraints every milestone inherits.

## Milestones

```
M0  shell       tray, hidden window, global hotkey, show/hide, CI matrix
M1  core        registry + manifest v1 + view protocol v1 + search/rank
                + SQLite; first built-in extension: applications
M2  builtins    command invocation + system commands; clipboard-history
M3  plumbing    selection capture, paste + focus restore, template engine
                -> snippets + quicklinks
M4  windows     window-management extension
M5  keys        hotkey binding UI, conflict detection, hyperkey
M6  ai          BYOK providers, auth.json, streaming, user-defined AI commands
M7  polish      permission onboarding, autostart, release workflow
M8  3rd party   script commands, then a sandboxed extension runtime
```

### M0 - shell

Tray application, single instance, a borderless always-on-top window created
hidden at startup, one global shortcut that toggles it, correct positioning on
the active monitor, and no dock or taskbar presence. Zero features.

The point is to prove the latency budget before any feature depends on it. Also
establishes the CI matrix so Windows breakage is caught immediately.

Exit criteria: hotkey to painted window under 80ms, measured on both platforms.

### M1 - core

The architectural keystone. Command registry, extension manifest v1, view
protocol v1, the search pipeline (providers, merge, fuzzy match, frecency
ranking), SQLite with migrations and the syncable-record convention, and the
Svelte rendering shell with a view stack.

`applications` ships as the first built-in extension and is the guinea pig that
proves the contract. Needs a real app indexer on both platforms, including the
`shell:AppsFolder` route on Windows.

Exit criteria: a usable daily-driver app launcher, built entirely on the same
contract third-party extensions will later use.

### M2 - builtins

Two built-in extensions: `system` commands (lock, sleep, empty trash, quit app)
and `clipboard-history` (watcher service, store, privacy exclusions, images).

This milestone was planned as needing no new plumbing. That was wrong. M1 built
the shape of the command contract but never the path that runs a command:
nothing invokes a command and nothing emits a view tree, because `applications`
contributes root items rather than commands. So M2 splits in two, and the first
change builds invocation with `system` as its first consumer, the way M1 paired
the contract with `applications`.

`calculator` is dropped. The author does not use one.

### M3 - plumbing

The real keystone for everything after it. Selection capture, paste into the
frontmost app, focus restore, and a shared template and placeholder engine.

Ships `snippets` and `quicklinks` as its first consumers. M4, M5, and M6 all
depend on parts of it.

### M4 - windows

The `window-management` extension: halves, quarters, thirds, maximize, center,
move to next display, and per-command hotkeys. Straightforward on Windows,
gated behind the Accessibility permission on macOS.

### M5 - keys

Bind any command to a global hotkey, with a recorder UI and conflict detection.
Then hyperkey as a background service.

Check first whether PowerToys Keyboard Manager already covers the Windows side.
Injected modifiers from a low-level hook have real edge cases with games and
raw-input applications, and this is a hobby project.

The recorder and settings UI were dropped in favour of JSON config files kept in
the user's dotfiles: `add-config-file` (config.json with live reload),
`add-file-backed-records` (snippets and quicklinks as text files),
`add-command-hotkeys` (per-command global hotkeys with conflict detection),
`add-hyperkey` and `add-hyperkey-tap` (CapsLock as the hyper modifier, tap for
Escape), and `add-keyword-expansion` (type a keyword, get the snippet).

Status: shipped and verified on both platforms. The macOS side is a `hidutil`
remap of CapsLock to F18 with a `CGEventTap` over it, because a tap alone cannot
own CapsLock: the lock state and its debounce live below every tap location.

Two verification items stay open, both recorded in their tasks with what was and
was not exercised. `add-file-backed-records` 3.1 (creating a record through the
launcher form) could not be driven on macOS, because synthesized keystrokes
reach the webview out of order and a synthesized Tab is typed rather than moving
focus. `add-command-hotkeys` 4.5's OS-refusal case cannot occur on macOS at all:
Carbon accepts a chord the system already owns and simply loses at dispatch,
where Windows refuses the registration outright.

### M6 - ai

Bring-your-own-key AI commands: highlighted text in, transformed text out.
A small provider trait with a native Anthropic adapter plus an
OpenAI-compatible adapter that covers OpenRouter, Ollama, and most others.
Keys in the OS keychain. Inference runs in Rust, never in the webview.
Streaming output through the existing full-tree-replace protocol.

User-defined AI commands come almost free from the M3 template engine: a named
prompt template with placeholders, a hotkey, and an output action.

Explicitly not a chat interface. One-shot transforms only.

Shipped as one change, `add-ai-commands`. The `dango.ai` extension contributes
nine transforms (Improve Writing, Fix Spelling and Grammar, Make Shorter, Make
Longer, Make Simpler, Make Professional, Summarize, Explain This, Translate),
each one a prompt template the config file can override, disable, or add to.
Commands come from `config.json`, so a new prompt is a command with a hotkey and
an alias, rebuilt live on an edit. Providers are Anthropic, any OpenAI-compatible
endpoint, and Ollama on the machine, resolved per command along with the model
and the thinking level. The provider adapters are the `genai` crate behind a thin
trait rather than two hand-rolled ones.

Two deliberate departures from the plan above. Keys live in plain `auth.json` in
the config directory rather than the OS keychain: one user with two machines is
better served by a file that can be copied, and the constraint in `config.yaml`
was amended to say so. And the answer is shown by default rather than pasted;
pasting and copying are per-command settings and actions on the result.

A fourth provider kind arrived with it that the plan above did not have: `cli`,
which runs a program that takes a prompt and prints an answer. It is how a
ChatGPT or Claude subscription pays for a transform, through the client the
vendor ships and the user has already signed in to, rather than through API
credit. It needs no key, streams what the program prints, runs it in an empty
directory, and takes the whole process tree down when an answer is abandoned.

Left out: markdown rendering of the answer, model discovery, and any accounting
of tokens or cost.

Carried forward, unverified: no hosted provider has been exercised on either
platform, because there is no key with credit on one. Ollama and the `cli` kind
were both driven end to end on Windows and macOS, so what is untested is
narrowly the request path to a hosted endpoint. The failures around it are
covered: missing key, rejected key, unknown model, unreachable and rate limited
all have tests, and the missing-key message was shown live on both platforms.
Tasks 6.1 and 6.2 of `add-ai-commands` stay unticked for this, and are the first
thing to run once a key exists.

### M7 - polish

Permission onboarding (a genuine UX surface on macOS), autostart, and the
release workflow that produces installers for both platforms. Code signing and
auto-update are deliberately deferred and may never happen, since there are no
external users.

The preferences window was cut from this milestone. M5 settled on JSON config
files in the user's dotfiles as the settings surface, and a window that edits
the same values is not part of this MVP.

### M8 - third party

Script commands first: a shell script with a metadata header, run through the
existing registry. Cheap, useful, and it validates the contract from outside the
Rust codebase.

A real sandboxed runtime comes after, and the choice between WASM and an
embedded JS engine stays open until then.

## Deferred, with reasons

| Item                                    | Why not                                              |
| --------------------------------------- | ---------------------------------------------------- |
| Calculator                              | Dropped from M2. The author does not use one.        |
| Linux and Wayland                       | No Linux desktop. Wayland would break four features. |
| Accounts and cloud sync                 | Weeks of work so two machines agree on snippets.     |
| Preferences window                      | Config files in dotfiles won in M5. Not in this MVP. |
| Sync between machines                   | Not in this MVP. Record discipline keeps it open.    |
| Code signing, notarization, auto-update | No external users.                                   |
| AI chat interface                       | A different product. Transforms first.               |
| Grid views, menu-bar commands           | Add once a real command needs them.                  |
| Elevated window support on Windows      | Hard ceiling of a non-elevated process. Accept it.   |
