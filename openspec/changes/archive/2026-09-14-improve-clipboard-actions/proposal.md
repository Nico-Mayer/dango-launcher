## Why

Two things get in the way of using clipboard history every day.

First, Ctrl+K does two jobs at once. The Bits UI `Command` primitive ships with
its vim-style bindings on, so Ctrl+J and Ctrl+K move the selection, and the
launcher also binds Ctrl+K (and Cmd+K) to open the action panel. One press moves
the selection up a row and then opens the panel for the wrong row. The list has
arrow keys for moving; the chord should only ever mean "show me the other
actions".

Second, confirming a history entry only puts it back on the clipboard. The user
then has to switch back and paste themselves, which is two more steps for the
thing they open the history to do. Enter should put the entry where they were
working, the launcher should get out of the way as it does so, and the user who
really does want "copy" as the default should be able to say so in the config
file.

Milestone: M2 - builtins. It finishes the clipboard-history extension M2
delivered, using the paste plumbing M3 added.

## What Changes

- Ctrl+J, Ctrl+K, Ctrl+N, and Ctrl+P no longer move the selection in any list.
  Only the arrow keys do. This is a change to root search and to pushed list
  views alike.
- Ctrl+K on Windows and Cmd+K or Ctrl+K on macOS open the action panel for the
  selected row and do nothing else. **BREAKING** for anyone who had learnt the
  vim chords, which were never documented.
- Clipboard history gains a paste action that puts the chosen entry on the
  clipboard and pastes it into the application the user was in, text or image.
  The entry stays on the clipboard afterwards, as Raycast does it, so the user
  can paste it again.
- Enter on a history entry runs paste by default. A new extension preference,
  `primary-action`, set to `copy` in `config.json` makes Enter copy the entry to
  the clipboard instead. The other action stays one step away in the action
  panel.
- Both paste and copy hide the launcher as they run, so the user is back in
  their application before the text lands.
- The footer and the action panel name the chord for the platform the user is
  on: Cmd on macOS, Ctrl on Windows. In a list view the footer names the
  selected row's primary action instead of a generic "Select".
- The remove action keeps the list open, as it does today.

## Capabilities

### New Capabilities

<!-- None. -->

### Modified Capabilities

- `view-protocol`: the action panel requirement gains the chord that opens it,
  the rule that Ctrl with J, K, N, or P never moves a list's selection, and the
  requirement that the advertised chord and primary action match the platform
  and the selected row.
- `clipboard-history`: the browsable-and-restorable requirement changes so that
  choosing an entry pastes it by default, the primary action is configurable,
  and both paste and copy hide the launcher.
- `selection-and-paste`: the clipboard-survives requirement gains its one
  exception, a pasted history entry, which is meant to stay on the clipboard.

## Impact

- `src/App.svelte` and `src/lib/ProtocolView.svelte`: the `Command.Root`
  primitive gets its vim bindings turned off. One prop per file, on top of the
  archived `add-list-navigation` change.
- `src/lib/platform.ts` (new), `src/App.svelte`, `src/lib/ActionPanel.svelte`:
  modifier names rendered per platform, and the footer's primary label taken
  from a list's items.
- `src-tauri/src/extensions/clipboard/extension.rs`: a new `insert` action, a
  `primary-action` preference, and action ordering read from it. The extension
  gains a text target and a preference reader.
- `src-tauri/src/text/mod.rs`: a second entry point that puts given clipboard
  content, text or image, on the clipboard and pastes it without restoring what
  was there before.
- `src-tauri/src/lib.rs`: wiring the text exchange and a preference reader into
  the clipboard extension.
- `docs/config.example.jsonc`: documents `primary-action`.
- No change to the view protocol's shape or version, and no new dependency. The
  preference is additive to the extension manifest.

Platforms: macOS and Windows. The chord fix is frontend-only and has no
platform branch. Paste runs through the existing per-platform text plumbing, so
it inherits the macOS Accessibility gate and the Windows foreground handoff, and
is verified on both.

## Non-goals

- No shortcut for the non-primary action beyond the action panel. Cmd+Enter or
  Ctrl+Enter for "the other one" is a separate decision.
- No change to the remove action, to filtering, or to what the history records.
- No pasting of an image on a platform where the paste chord cannot deliver
  one; the image goes on the clipboard and the paste key is sent, and what the
  target application does with it is its business.
- No settings surface. The preference is set in `config.json` like every other.
