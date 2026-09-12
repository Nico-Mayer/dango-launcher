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
M6  ai          BYOK providers, keychain, streaming, user-defined AI commands
M7  polish      preferences window, permission onboarding, autostart
M8  3rd party   script commands, then a sandboxed extension runtime
M9  sync        file-based, deliberately small
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

### M6 - ai

Bring-your-own-key AI commands: highlighted text in, transformed text out.
A small provider trait with a native Anthropic adapter plus an
OpenAI-compatible adapter that covers OpenRouter, Ollama, and most others.
Keys in the OS keychain. Inference runs in Rust, never in the webview.
Streaming output through the existing full-tree-replace protocol.

User-defined AI commands come almost free from the M3 template engine: a named
prompt template with placeholders, a hotkey, and an output action.

Explicitly not a chat interface. One-shot transforms only.

### M7 - polish

Preferences window, permission onboarding (a genuine UX surface on macOS),
autostart. Code signing and auto-update are deliberately deferred and may never
happen, since there are no external users.

### M8 - third party

Script commands first: a shell script with a metadata header, run through the
existing registry. Cheap, useful, and it validates the contract from outside the
Rust codebase.

A real sandboxed runtime comes after, and the choice between WASM and an
embedded JS engine stays open until then.

### M9 - sync

Deliberately minimal. An export file in a synced folder, or a git repository.
Accounts, a server, and end-to-end encryption are not worth it for one person
with two machines. The `updated_at` and `deleted_at` discipline from M1 keeps a
real sync engine possible if that ever changes.

## Deferred, with reasons

| Item                                    | Why not                                              |
| --------------------------------------- | ---------------------------------------------------- |
| Calculator                              | Dropped from M2. The author does not use one.        |
| Linux and Wayland                       | No Linux desktop. Wayland would break four features. |
| Accounts and cloud sync                 | Weeks of work so two machines agree on snippets.     |
| Code signing, notarization, auto-update | No external users.                                   |
| AI chat interface                       | A different product. Transforms first.               |
| Grid views, menu-bar commands           | Add once a real command needs them.                  |
| Elevated window support on Windows      | Hard ceiling of a non-elevated process. Accept it.   |
