## Why

Three defects made acting on a result unreliable, and each of them failed
silently, which is what made them hard to place. They were found while chasing
one report - "clipboard history is broken" - and are recorded here because the
specs said nothing about any of them, so nothing was violated on paper while the
feature did not work.

1. A view opened by a command's own hotkey accepted no actions at all. The
   frontend learnt the owning extension only from what `invoke_command`
   returned, and a hotkey never goes through it, so every action in that view
   was dropped before it was sent. Enter on a clipboard history entry did
   nothing, with no message.
2. A failure from an action that hides the launcher was usually lost. The paste
   path hides first and can only report afterwards, and the reset the hide fires
   cleared the message, so about half the time a failed paste looked like
   nothing happening.
3. Every image Dango put on the clipboard was recorded as a fresh entry. macOS
   re-encodes an image on the pasteboard - a 1571 byte favicon reads back as
   5427 bytes - and own-write suppression compared raw bytes, so each pasted or
   copied image left a duplicate behind.

The code for all three is already on `main`; this change is the specification
catching up with it.

Milestone: M2 - builtins, whose clipboard-history extension is where all three
surfaced. It adds nothing to the roadmap.

## What Changes

- A view tree is delivered together with the extension that owns the command
  that produced it, so a view behaves the same however the command was started.
- A hotkey-opened view accepts Enter, its per-action shortcuts, and its action
  panel, exactly as the same view does when it is opened from root search.
- A failure raised by an action that hid the launcher survives the reset and is
  shown on the next activation. Typing, running another action, or pressing
  Escape clears it.
- An image Dango puts on the clipboard is recognised as its own write even when
  the system hands it back re-encoded, so it does not enter the history.

## Capabilities

### New Capabilities

<!-- None. -->

### Modified Capabilities

- `command-invocation`: a delivered view tree names the extension that owns it,
  so an action from that view reaches its command.
- `command-hotkeys`: "the same way root search does" is stated to include the
  view's actions, not only its appearance.
- `launcher-shell`: the reset on hide keeps an unread failure, which is the one
  thing that does not return to its initial state.
- `clipboard-history`: Dango's own write is not recorded even when the system
  re-encodes it, which is what images always get.

## Impact

- `src-tauri/src/lib.rs`: the render event carries `{ owner, tree }`;
  `invoke_command` no longer returns the owner.
- `src/App.svelte`, `src/lib/types.ts`: the view stack takes its owner from the
  render event; a failure survives `resetToRoot`.
- `src-tauri/src/extensions/clipboard/watcher.rs`,
  `src-tauri/src/extensions/clipboard/source.rs`: own-write suppression matches
  an image by size and recency rather than by bytes.
- No new dependency. The extension-facing view protocol is untouched: the owner
  travels in the event envelope, not in the tree, so `protocolVersion` stays 1.

Platforms: macOS and Windows. The image re-encoding was measured on macOS;
suppression by bytes alone is fragile on either platform, and the fix has no
platform branch. Windows verification still happens on Windows.

## Non-goals

- No change to what an action does, only to whether it is delivered and
  reported.
- No change to the extension-facing view protocol or its version.
- No retrospective clean-up of duplicate image entries already in a history.
- No new diagnostics or logging beyond the failure message already shown.
