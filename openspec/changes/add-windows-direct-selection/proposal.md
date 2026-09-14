## Why

Part of **M3 - plumbing**, retrofitted. M3 recorded that "reading the current
selection is direct on macOS via `AXSelectedText`, but on Windows the realistic
path is copy, read clipboard, restore clipboard". The Windows half of that is
worse than slow: it is destructive.

Dango reads a selection on Windows by synthesizing Ctrl+C. That assumes Ctrl+C
means copy. In a modal editor it does not. Zed in Helix mode binds Ctrl+C to
toggle comments, so running Improve Writing on a selected block comments the
block out and copies nothing: the user gets "Select some text first" and a
mangled buffer. Helix's own default binds Ctrl+C the same way, so this is the
rule for that family of editors, not one app's quirk.

Every feature that reads a selection is affected, not just AI: a snippet or a
quicklink whose template uses `{{ selection }}` does the same thing. The bug was
found while verifying M6, but it belongs to M3's plumbing.

Windows does have a route that sends no keystrokes. UI Automation exposes the
focused element's selected text directly, which is what macOS already does
through the accessibility API, and the `DirectSelection` trait the code has is
already the seam for it - it simply has no Windows implementation.

## What Changes

- `DirectSelection` gains a Windows implementation over UI Automation: ask the
  focused element for its text pattern and read the selected range. No
  keystrokes, no clipboard, nothing the target application can mistake for a
  command of its own.
- The existing order holds on both platforms: the direct route first, the
  clipboard round trip only when the application does not answer. Windows stops
  being the platform where the fallback is the only path.
- A guard for the applications that still fall through: the user can name
  applications where the keystroke fallback must not be attempted. Dango reports
  that it cannot read the selection there rather than pressing a chord that means
  something else. That is the honest answer for an editor that exposes nothing.
- When the focused element answers UI Automation but reports no selection, that
  is an empty selection, not a reason to fall back. Only an application that
  cannot be asked at all reaches the keystroke path.

## Capabilities

### Modified Capabilities

- `selection-and-paste`: the Windows read is no longer defined as a clipboard
  round trip. The requirement gains the direct route on Windows, the rule for
  when the fallback is used, and the user's ability to refuse the fallback for a
  named application.

## Impact

- **Affected code**: `src-tauri/src/platform/windows/` gains a `DirectSelection`
  implementation; `src-tauri/src/platform/mod.rs` passes it to the exchange, the
  way macOS already passes `MacSelection`; `src-tauri/src/text/mod.rs` learns the
  fallback rule and the exclusion check; `src-tauri/src/config/` gains the
  excluded-applications setting for it.
- **Dependencies**: a UI Automation binding for Rust. The design names the
  candidates and picks one.
- **Platforms**: Windows is where the work is. macOS is unchanged and must be
  verified as unchanged, since the shared exchange code around the trait moves.
- **Risk to verify first**: whether Zed exposes UI Automation text patterns at
  all. If it does not, this change stops Zed mangling the buffer but does not
  give it working selection reads, and the guard is what the user gets. The first
  task is a spike that answers this before the rest is built.

## Non-goals

- **No accessibility work beyond reading a selection.** Not an accessibility
  layer, not a screen-reader surface, not element inspection for anything else.
- **No change to how text is inserted.** Paste still goes through the clipboard
  and the keystroke; only reading changes.
- **No per-application configuration beyond the exclusion list.** No profiles, no
  per-app strategies, no key remapping.
- **No change on macOS.** The direct route there already works.
- **No Linux.**
