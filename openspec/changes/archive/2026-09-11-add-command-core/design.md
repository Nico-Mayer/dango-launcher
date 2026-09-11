## Context

M0 leaves a resident tray application with a warm hidden window and a working
global shortcut, and nothing else. This change fills that window and, in the
process, fixes the two contracts that later milestones cannot renegotiate
cheaply: the extension manifest and the view protocol.

That is the tension shaping every decision here. The contracts want to be small,
because everything added to them must be supported forever, including by a
sandboxed third-party host that does not exist yet. But they must be exercised
hard enough now to expose their flaws, which means shipping a real feature
against them rather than a placeholder.

The 80ms activation budget from M0 also becomes a live constraint rather than a
measurement. A registry, a search pipeline, a database, and an application index
all now sit between startup and a usable prompt.

## Goals / Non-Goals

**Goals:**

- Fix manifest v1 and protocol v1, small enough to keep and versioned so they
  can grow.
- Prove both contracts with `applications`, a feature the author uses daily.
- Keep activation within budget with the full pipeline loaded.
- Make the platform trait boundary carry real weight on its hardest case.

**Non-Goals:**

- Designing the sandboxed host. Only the boundary it will plug into.
- A settings surface for enable and disable. The lifecycle exists; the UI is M7.
- Making the protocol expressive. Three view kinds. Grid, arguments, and
  menu-bar commands wait for a command that needs them.
- Optimising search below the stated budgets.

## Decisions

### Everything is a command, and the registry does not know about built-ins

The registry stores commands, not implementations. Each command carries the
identity of the host that executes it. Today there is one host, the built-in
native host; M8 adds a script host and later a sandboxed host, and neither
requires the registry to change.

This is the whole bet of the architecture, so it is worth being explicit about
what it buys: when the third-party host arrives, the manifest, preferences,
hotkeys, aliases, enable and disable, ranking, and rendering already work and
have been used daily for months. What is left is the sandbox itself.

**Rejected:** running built-ins through the eventual sandbox for purity. A
clipboard watcher, an accessibility-based window mover, and a keyboard remapper
cannot live in a sandbox. Either built-ins lose the operating system access that
makes them useful, or the sandbox acquires so many escape hatches that it stops
being one. Built-ins are extensions in every way the user can see, and native in
the one way they cannot.

### Four contribution types, and services are the one built-ins keep

`commands`, `rootItems`, `services`, `preferences`. The set is deliberately
small and each earns its place by being a shape the others cannot express.

`services` is the type third-party extensions will never get, because a
background process with operating system access is precisely what the sandbox
exists to prevent. Establishing that asymmetry now, while only built-ins exist,
avoids having to take a capability away later.

The types were chosen against real upcoming features rather than in the
abstract. Snippets need all four: management commands, individual snippets as
root items, a background expander, and preferences. That one feature spanning
the whole model is reasonable evidence the model is right.

### Full-tree replace, no reconciler

A command is a state machine that emits a complete view on every change. There
is no patching and no diffing.

The reason this holds is arithmetic. Views are bounded at tens of items, and the
worst case in the roadmap is streaming AI output at perhaps thirty updates per
second, which is thirty small JSON messages per second. Serialising a whole tree
at that rate is nothing. A reconciler is the single heaviest piece of Raycast's
extension architecture and, at this scale, it would buy nothing.

The cost is that identity has to be explicit: list items are keyed by
identifier, so replacing a tree preserves selection and scroll instead of
resetting them. That is a small burden on command authors and it is stated in
the protocol rather than left to be discovered.

### Filtering ownership is declared per view

By default the launcher filters a list against the query and the command is not
re-invoked. A command that needs search-as-you-type declares that it filters
itself and receives query changes.

Copied from Raycast, because the split is correct: most commands have a fixed
set of items and re-running them per keystroke would be wasteful, while the ones
that query a remote service must own filtering. Making it a declaration rather
than an inference keeps the frontend simple.

### Providers get a deadline, and the list is never blocked

Every root items provider is queried asynchronously with a 50ms budget. Results
stream in. A provider that is slow, that errors, or that never returns cannot
delay anything else.

This is the most likely long-run failure of an extension-based launcher: one
badly written extension makes the whole thing feel broken. It is nearly free to
design in now and would require rewriting the pipeline to add later.

Cancel-and-restart per keystroke rather than debouncing. Debouncing trades
responsiveness for fewer queries, which is the wrong trade when queries are
local and cheap.

### Frecency is stored, matching is not

Fuzzy matching runs in memory against the candidate set on every keystroke,
because it must reflect the exact current query. Frecency is a persisted
per-item score updated on launch and decayed by age.

Keeping them separate means the ranking formula can be tuned without a
migration, and the expensive part, matching, never touches the database.

### The index is background work, always

Application indexing never runs on the activation path. It runs after startup,
persists to the store, and refreshes on a schedule and on filesystem or shell
notification. The launcher is usable during the first build, with applications
appearing as they are found.

This is what protects the M0 latency budget from every future feature: the rule
is that activation reads memory, and everything expensive happens before or
after it.

### Windows indexing goes through the shell, not the Start Menu

Enumerating the shell's applications folder yields packaged and unpackaged
applications together, with the identifiers needed to launch each and icons
obtainable at any size. Scanning Start Menu shortcut files is the obvious
approach and it silently misses every Store application, which is the trap most
launchers fall into first.

This is the hardest platform-specific work in the change and needs a spike
before the rest is built.

**Spike results** (`src-tauri/examples/apps_spike.rs`, release build, 136
entries on the author's Windows 11 machine): enumerating through `IShellItem`
and `IEnumShellItems` with display name, `System.AppUserModel.ID`, and
`System.Link.TargetParsingPath` read per entry takes about 230ms. Every entry
has an AppUserModelID, in four shapes: `Family!App` for packaged apps, a
registered string such as `Brave.IL25…` or `308046B0AF4A39CB` for Win32 apps
that declare one, `Microsoft.AutoGenerated.{GUID}` for plain shortcuts, and a
known-folder-relative path such as `{GUID}\cmd.exe` for system tools. Packaged
entries have no link target; every other entry does. Icons come from
`IShellItemImageFactory::GetImage` at 64px as premultiplied BGRA, all 136 in
under a second with no failures. `ShellExecuteW` on `shell:AppsFolder\<AUMID>`
launches every shape, and takes 200 to 260ms, so it must run off the UI thread.

Two decisions follow from what the spike showed.

**Launching must find the running instance itself.** Windows does not
deduplicate: launching Calculator a second time started a second process, and
whether a Win32 app reuses its instance is up to the app. To meet the
already-running scenario the extension first looks for a top-level window
belonging to the application, by comparing the window's own AppUserModelID
(read through `SHGetPropertyStoreForWindow`) with the entry's, and for Win32
entries also by comparing the owning process's image path with the link target.
A match is brought forward with the foreground-lock handling from M0; only a
miss launches.

**The shell folder is filtered by target, not by name.** It contains shortcuts
that are not applications: uninstallers pointing at `msiexec.exe` or
`unins*.exe`, `http` and `https` links, and documents such as `.html` and
`.ini`. Entries are kept when the target is an executable, a `.msc` console, a
shell namespace object (`::{GUID}`), or a non-web URL scheme such as
`steam://`, and dropped otherwise. Name patterns like "Uninstall" are not used
as the rule, since they are localised.

### Drop the whole thing behind an `AppIndexer` trait

macOS reads bundle metadata from the filesystem; Windows talks to a shell
interface. They share nothing but their output shape. The trait returns a
uniform indexed application, and everything above it, ranking, icons, launching,
and actions, is platform-neutral.

## Risks / Trade-offs

- **Manifest v1 will be wrong somewhere.** Five milestones of built-ins will
  find it. Versioning is the mitigation, and the willingness to ship v2 rather
  than contort features to fit v1.
- **The change is large.** It is the keystone, and splitting the contract from
  its first consumer would mean designing the contract against a guess. The task
  groups are ordered so the application stays runnable throughout.
- **Windows shell enumeration is the schedule risk.** It is unfamiliar
  interop-heavy work. Spiked first, so a bad surprise arrives before the rest is
  built on top of it.
- **The 80ms budget may fail once everything loads.** The defence is the rule
  that activation touches only memory. If it fails anyway, startup work moves
  later rather than the budget moving up.
- **Frecency needs tuning and will feel wrong at first.** Accepted. The formula
  is isolated so it can be adjusted without touching the pipeline.
- **Three view kinds may prove too few.** Deliberate. Adding a view kind when a
  command needs it is easy; removing one that nothing uses is not.
