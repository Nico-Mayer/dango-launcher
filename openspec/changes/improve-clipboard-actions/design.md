## Context

See proposal.md - Why for the motivation. What shapes the approach:

- Both list surfaces are Bits UI `Command.Root` (`src/App.svelte` for root
  search, `src/lib/ProtocolView.svelte` for a pushed list). The primitive has a
  `vimBindings` prop that defaults to `true`; checked in
  `node_modules/bits-ui@2.19.2`, `dist/bits/command/command.svelte.js` line
  848: `const isVim = this.opts.vimBindings.current && e.ctrlKey`, after which
  Ctrl+J and Ctrl+N act as Arrow Down and Ctrl+K and Ctrl+P as Arrow Up. The
  launcher's own `onKeydown` in both files opens the action panel on
  `k` with `ctrlKey || metaKey`. The primitive's handler runs on the root
  element and the launcher's on `window`, so on Ctrl+K the selection moves
  first and the panel then opens for the new row.
- The clipboard extension (`src-tauri/src/extensions/clipboard/extension.rs`)
  offers two actions per entry: `restore`, titled "Copy to Clipboard", which
  calls `Watcher::restore` and returns `ActionOutcome::Done`, and `remove`,
  which returns `Replaced`. `Done` makes `run_action` in `lib.rs` call
  `hide_after_launch`, so copy already hides the launcher from the backend.
- `Watcher::restore` marks the write as Dango's own before making it, so the
  watcher neither records it nor reorders the history. `Watcher` also
  implements the text module's `OwnWrites` trait with the same marking.
- The extension is built as `ClipboardExtension::new(history, watcher)` with no
  preference reader and no text target. Its four preferences are read through
  `PreferencePolicy(Preferences)`, a separate object handed to the watcher.
- Pasting lives in `src-tauri/src/text/mod.rs`. `TextExchange::insert(text,
  caret)` saves the clipboard, dismisses the launcher, yields the foreground,
  marks and writes the text, sends the paste chord, waits for it to settle, and
  restores the saved clipboard. `TextTarget` is the trait an extension depends
  on; the snippets extension holds it as `Option<Arc<dyn TextTarget>>`, `None`
  when the platform has no key injection. The exchange is `app.manage`d in
  `lib.rs` right before the clipboard extension is constructed.
- `Content` is the clipboard's own `Text | Image` enum, and the exchange
  already writes both kinds in `restore`.
- Preferences are read live: `Preferences` reads through the store on every
  call, and the config file store is re-applied on change. Nothing needs
  caching or invalidating.

## Goals / Non-Goals

**Goals:**

- Remove the chord collision by configuration of the primitive, not by
  patching key handling.
- Give clipboard history a paste that reuses the platform handoff, the
  permission gate, and the own-write marking the text plumbing already has.
- Match Raycast: the pasted entry becomes the current clipboard.
- Make the primary action a preference like the extension's other four.

**Non-Goals:**

- A shortcut for the non-primary action.
- Moving a pasted entry to the top of the history. The existing spec says
  Dango's own writes never reorder it, and that stays true.
- Any change to what `run_action` returns or to the view protocol's shape.

## Decisions

### Turn the primitive's vim bindings off

`vimBindings={false}` on both `Command.Root`s. That is the switch the primitive
offers for exactly this, so nothing is patched and nothing new listens for
keys. The launcher's Ctrl/Cmd+K handler is unchanged and becomes the only thing
that reacts to the chord.

Alternatives rejected:

- Stop propagation of Ctrl+K in the launcher's handler before the primitive
  sees it. The launcher's handler is on `window`, in the bubble phase, and the
  primitive's is on the root element, so the primitive fires first. A capture
  handler would work but would be patching around a prop.
- Move the action panel to another chord. The chord is fine; it is the second
  meaning that is wrong, and Ctrl+J/N/P would still move the list, which no one
  asked for.

### Paste is a new action, `insert`; `restore` stays and keeps its title

`ACTION_INSERT = "insert"`, titled "Paste to Active App", is added alongside
`restore` ("Copy to Clipboard") and `remove`. Action ids are what the frontend
sends back and what a per-command hotkey might one day name, so the existing id
is not renamed. The order of the first two is decided per list build from the
preference; `remove` is always last.

The spec uses the words `paste` and `copy` for the preference values because
that is what the actions are called on screen. The value `paste` maps to
`ACTION_INSERT`; the action id follows the snippets extension, where the same
operation is called `insert`.

Alternatives rejected:

- Make `restore` itself paste when the preference says so. One id with two
  behaviours means the action panel cannot list both, and the panel is the whole
  point of a secondary action.
- Title the action "Paste" alone. The panel also lists "Copy to Clipboard", and
  the two need to read as opposites: where the text goes, not whether it is
  pasted.

### Paste is a second entry point on the text exchange that does not restore

`TextTarget` gains `fn paste_content(&self, content: &Content) -> Result<(),
TextError>`. `TextExchange` implements it as the insert round trip without the
save and restore steps: permission check, dismiss, yield to the previous
application, mark the write as Dango's own, put the content on the clipboard,
send the paste chord, settle. The entry is left on the clipboard, which is the
Raycast behaviour the proposal asks for and what the `selection-and-paste`
delta now allows. `insert(text, caret)` is untouched.

The clipboard extension holds `Option<Arc<dyn TextTarget>>`, exactly as the
snippets extension does. Its `insert` action is:

```rust
ACTION_INSERT => match (&self.text, self.history.content(item_id)) {
    (None, _) => ActionOutcome::Failed(TextError::PermissionMissing.to_string()),
    (Some(text), Ok(content)) => match text.paste_content(&content) {
        Ok(()) => ActionOutcome::Done,
        Err(error) => ActionOutcome::Failed(error.to_string()),
    },
    (_, Err(error)) => ActionOutcome::Failed(error.to_string()),
}
```

`Done` then reaches `hide_after_launch` in `lib.rs` as every other success
does. The exchange has already dismissed the launcher before the paste, so the
second hide is the no-op `hide()` already guards for.

Order matters for the failure scenarios. The permission check comes first and
the yield comes before the clipboard write, so a missing permission or a
missing application to return to leaves the clipboard untouched. Only a paste
keystroke that fails after the write leaves the entry on the clipboard, which
is the harmless outcome: the user can paste it themselves.

The own-write marking goes through the exchange's `OwnWrites`, which in
production is the watcher, so the pasted entry is neither recorded again nor
moved to the top, the same as `restore` today.

Alternatives rejected:

- Call `Watcher::restore` in the extension and then have the exchange send
  only the paste chord. It splits one operation across two modules and puts
  the clipboard write before the permission check, so a denied paste on macOS
  would still change the clipboard.
- Generalise `insert` to `Content` and restore the previous clipboard
  afterwards, as a snippet does. Rejected by the user: the pasted entry should
  stay on the clipboard, as in Raycast.
- Reuse `insert(text, caret)` for text entries. It restores the clipboard,
  which is the behaviour being ruled out, and it cannot carry an image.

### `primary-action` is an extension-level string preference

Declared in the manifest next to the other four: `PreferenceDecl { command_id:
None, key: "primary-action", kind: String, default: Some("paste"), required:
false }`. Set in `config.json` at
`extensions.dango.clipboard.preferences.primary-action`. The extension reads it
each time it builds the list, so a config edit applies the next time the
history opens. Any value other than `copy` is read as `paste`.

The manifest gains one optional preference. That is additive and needs no
manifest version bump.

Alternatives rejected:

- Scope it to the `history` command (`commands.history.preferences`). The
  config store supports it, but the extension has one command, its other
  preferences are extension-level, and `Preferences::string` reads only
  extension-level keys today. Command scope would add a reader method for one
  value.
- A boolean such as `paste-on-enter`. Two named values read better in a config
  file than `true` meaning paste.
- Fail loudly on an unknown value. The configuration spec's stance is that a bad
  value falls back rather than takes anything down; the history opening with
  the default is that stance applied.

### The extension gets its reader and text target at construction

`ClipboardExtension::new(history, watcher, preferences, text)`. `lib.rs` already
builds a `Preferences` for the policy; a second reader over the same
declarations and store is cheap and keeps the extension from reaching through
the watcher's policy for a value that is not the watcher's business. The text
target is the `Option<Arc<TextExchange>>` `lib.rs` already has in hand when it
constructs the extension.

### Hiding is not a new mechanism

Copy already hides via `Done`. Paste hides via the exchange's `launcher.dismiss()`
before the paste chord, then again via `Done`. Nothing is added in the
frontend; `runViewAction` keeps awaiting `run_action` and receives `done` once
the paste has settled, by which point the window has been hidden for the length
of the handoff. The 100ms budget in the spec is the time to the hide, not to the
paste landing, which keeps its own 400ms budget in `selection-and-paste`.

## Risks / Trade-offs

- `vimBindings={false}` also drops Ctrl+N/P, which some users know from Emacs
  and shells → they were never advertised, the arrow keys are one row of keys
  away, and the spec now states the rule.
- Adding a method to `TextTarget` breaks the snippets test fake → it gets a
  one-line `paste_content` that records the content; the tasks name it.
- A pasted image whose target rejects images leaves the paste chord doing
  nothing visible, with the image now on the clipboard → the failure is the
  target's, not Dango's, and the image being on the clipboard is what the user
  chose. Recorded as a non-goal in the proposal.
- Leaving the entry on the clipboard is the one place an insertion is
  observable on the clipboard → the `selection-and-paste` delta names it as the
  single exception, so the rule stays checkable everywhere else.

## Migration Plan

None. The preference is optional with a default, the action ids that existed
keep their ids, and no stored data changes. Rollback is reverting the commit.
