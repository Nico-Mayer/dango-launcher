## Context

See proposal.md for the motivation. What shapes the approach:

- The `system` extension (`src-tauri/src/extensions/system/`) is four small
  no-view or view commands over one `SystemControl` trait, with a `FakeControl`
  in its tests. Its manifest declares a named Lucide icon per command, subtitle
  `System`, and the `red` tint. All six tints in `src/lib/tint.ts` are taken
  by the built-in extensions that exist.
- `tauri-plugin-opener` is already registered in `run`, and quicklinks open
  through it: `lib.rs` wraps the app handle in a `DefaultBrowser` struct that
  implements the `snippets::OpenUrl` trait, so the extension never sees Tauri.
  The plugin has an `open_path(path, with)` entry point beside `open_url`.
- The config file path comes from `config::config_path()`, honouring
  `$DANGO_CONFIG_DIR`. The loaded config is an `Arc<config::FileConfig>`
  shared with every extension that reads preferences. `FileConfig::persist`
  creates the directory, writes the file, and records the text in its
  own-write signal so `config::watch` ignores the write. A missing file is a
  valid empty configuration and Dango never creates one unasked.
- A no-view command reports success or failure through `InvocationContext`;
  success hides the launcher and a failure shows the banner with the message.
- Raw operating-system errors go to `dango.log` through `crate::log::append`;
  the banner gets a sentence the interface-copy spec allows.

## Goals / Non-Goals

**Goals:**

- One command, no new dependency, no new extension, no new tint, no change to
  the manifest or protocol version.
- The extension stays testable without Tauri: the file opener is a trait with
  a fake, and the file logic runs against a `FileConfig` in a temp directory.

**Non-Goals:**

- A general "open any file" facility or an editor preference. One trait
  method taking a path is all that is added.
- Checking whether an editor actually appeared. The opener reports whether the
  operating system accepted the request; what happens after is the OS's.

## Decisions

### The command lives in `system`, under `dango.system.open-config`

The alternative was a new built-in extension named after the launcher, as
Raycast does with "Open Raycast Preferences". That costs a manifest, a
registration block in `lib.rs`, a spec capability, and a seventh tint for one
command, when all six palette entries are already used. `system` is "what a
launcher is expected to have", already carries the red tint and the `System`
subtitle, and a setting-related command reads naturally beside Lock Screen
and Sleep. Rejected: the new extension. If more launcher-about-itself commands
arrive later (open log, reveal config directory, restart), that is the moment
to move them together.

Consequence: the alias and hotkey path is
`extensions.dango.system.commands.open-config`.

### Open through `tauri-plugin-opener::open_path`, with no `with` program

The plugin is registered and the project rule is to use it rather than shell
out. `open_path(path, None)` goes through the `open` crate's detached mode. On
macOS that is `/usr/bin/open -- <path>`, which uses the Launch Services
association for the file. On Windows it is a hidden `powershell.exe` running
`Invoke-Item -LiteralPath`, which is the shell's "open" verb for the file type,
with `explorer.exe <path>` as the fallback; a file type with no association
gets Windows' own chooser. Both are exactly "the OS default for the file
type" that was asked for, with no code of ours deciding what an editor is.

Alternatives considered:

- `open_url` with a `file://` URL. The path entry point exists for this and
  avoids URL-escaping a Windows path. Rejected.
- Shelling out to `open` or `cmd /c start`. Rejected by the standing rule and
  by `start`'s quoting rules on Windows.
- Passing an editor through `with` from a preference. Out of scope by the
  proposal; the trait method takes only a path so adding it later is one
  argument.

The opener returns once the handler was started, so a success means the
operating system took the request, not that a window appeared. That is the
boundary the spec draws, and it is why the Windows no-association chooser is a
success.

### A `system::OpenFile` trait, implemented in `lib.rs` on the app handle

Mirrors `snippets::OpenUrl`. The trait has one method, `open(&self, path:
&Path) -> Result<(), String>`. The existing `DefaultBrowser` struct in `lib.rs`
gains the second impl and is renamed `DefaultHandler`, since it now opens files
as well as URLs. `SystemExtension::new` takes the opener and the
`Arc<FileConfig>` as two more arguments; the command is declared in the
manifest, so making them optional would only move the failure from compile
time to the "declared but not invocable" test.

Alternative: add `open_path` to `SystemControl`. Rejected; that trait is
implemented per platform under `platform/`, where the app handle and the
plugin are not available, and the quicklink precedent already shows the
app-handle wrapper is the right seam for the opener plugin.

### `FileConfig::ensure_file` creates the minimal file through the own-write signal

A new method on `FileConfig` returns the file's path, creating the file first
when it is absent. The content is the in-memory configuration with `version`
set to 1 when it was unset, serialised by the existing `to_json`, so a machine
with no file gets:

```json
{
  "version": 1
}
```

The write goes through the same path as `persist`: directory created, text
recorded in `own_write`, then written. The watcher sees its own text and does
not reload, so the tray status does not flicker and nothing is re-applied. An
existing file, parseable or not, is not read or rewritten: only `exists` is
checked. The version default is set on the in-memory config as well, so a
later `set` writes `version: 1` too instead of dropping the one key the
created file had.

Alternatives considered:

- Fail with "there is no config file" and tell the user to create one.
  Rejected; the command exists to save that step.
- Copy `docs/config.example.jsonc` as a starting point. Rejected; the live
  file cannot carry comments and the example is documentation, not a default.
- Write the file with `std::fs::write` from the command. Rejected; the watcher
  would reload it as an edit and the write would bypass the one place that
  knows the own-write protocol.

### Icon

The command uses the Lucide `settings` gear, added to the registry in
`src/lib/Icon.svelte` as `settings`, imported individually like the others.

### User-facing text

Written with the ux-writing skill and checked against
`openspec/specs/interface-copy/spec.md`: Title Case for the command title,
sentence case elsewhere, each sentence under 20 words, what failed first and
where to look second, no identifiers or raw errors on screen.

- Command title: `Open Config File`
- Command subtitle: `System` (inherited from the extension)
- Keywords: `config`, `configuration`, `settings`, `preferences`, `keybinds`,
  `hotkeys`, `keys`, `edit`, `json`
- Banner, open failed: `Couldn't open the config file. See dango.log for the reason.`
- Banner, create failed: `Couldn't create the config file. See dango.log for the reason.`
- Log, open failed: `config: could not open <path>: <error>`
- Log, create failed: `config: could not create <path>: <error>`

README, configuration section, after the sentence naming the path:

> Run the Open Config File command from the launcher to open it in the app
> your system uses for `.json` files. It creates the file if there is none yet.

## Risks / Trade-offs

- [`.json` is associated with a browser or a viewer rather than an editor] →
  That is the user's association and the request as stated. The README names
  the association so the user knows what to change. No Dango-side editor
  choice.
- [PowerShell startup adds a visible delay on Windows before the editor
  appears] → The launcher has already hidden by then, so the delay is between
  the keypress and the editor window, not a frozen launcher. Accepted; it is
  what the plugin does for every path.
- [The editor may open behind the previous app on Windows because of the
  foreground lock] → The handler is spawned while Dango is the foreground
  process, which normally grants the new process foreground rights. Checked
  in the Windows verification task rather than worked around in advance.
- [The user saves a broken file from the editor] → Existing behaviour: the
  watcher keeps the last good configuration and reports through the tray and
  the log. Nothing new.
- [The config directory is read-only or missing and cannot be created] → The
  create step fails with the banner above and nothing is opened, so the user
  gets one failure rather than an editor on a file that does not exist.

## Migration Plan

Additive. No config key changes. Rollback is removing the command, the trait
impl, and the icon; a config file created by the command is a valid
configuration for any version.
