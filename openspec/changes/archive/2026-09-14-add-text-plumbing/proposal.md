## Why

Dango can read the clipboard and it can launch things, but it cannot put a
single character into the application the user was in a moment ago, and it
cannot see what they had selected. That one missing path is **M3 - plumbing**
from `openspec/ROADMAP.md`, and it is the keystone the roadmap says it is:
snippets, quicklinks, window management's per-command hotkeys, and every AI
transform in M6 are all the same round trip. Read what the user has, act on it,
give the text back to where they were.

Built once with two consumers proving it, the way M1 paired the command contract
with `applications` and M2 paired invocation with `system`. Built four times by
four features guessing at it is the alternative, and each guess would be wrong
in a different way on a different platform.

The template engine rides along for the same reason. A snippet, a quicklink, and
an M6 prompt are all a string with holes in it, and the holes are the same holes:
the clipboard, the selection, the date, a value the user is asked for. Deciding
once what a placeholder is worth more than three extensions each inventing a
syntax.

## What Changes

- A `SelectionSource` trait that reads the text selected in the frontmost
  application, with the platforms answering it differently: macOS asks the
  accessibility API directly, Windows copies, reads, and puts the clipboard back.
- A `TextInjector` trait that puts text into the frontmost application, through
  the clipboard and a synthesised paste shortcut, with the user's clipboard
  restored afterwards and the history left undisturbed.
- Focus restore becomes part of that path rather than something the launcher
  does on its way out. `restore_previous_focus` already exists on both platforms
  and is a deliberate no-op on macOS; paste has to happen after the launcher is
  gone, and that ordering is now specified rather than incidental.
- A shared template engine: one placeholder vocabulary (`clipboard`, `selection`,
  `date`, `uuid`, `cursor`, `argument`, `query`), one renderer, one parse step
  that reports which placeholders a template needs so a caller can ask for them.
- The form submit path is closed. `view-protocol` has required since M1 that a
  submitted form's values reach the command, and nothing implements it: the
  frontend's `onsubmit` is wired to an empty function. An argument placeholder is
  the first thing that needs it, so this change implements a requirement that
  already exists rather than adding one.
- A `snippets` built-in extension: named templates the user creates, kept in
  SQLite, offered in root search, expanded and pasted where the user was.
- A `quicklinks` built-in extension: named URL templates with a `query`
  placeholder, opened in the default browser.
- The macOS Accessibility permission becomes a surface the user can see rather
  than a silent failure. Both capabilities need it and neither works without it.
- Covered on Windows and macOS. macOS is built and verified first; the Windows
  half lands as code plus confirmation tasks that only CI and the author's
  Windows machine can close, the way the clipboard change did.

## Capabilities

### New Capabilities

- `selection-and-paste`: what reading the selection means on each platform and
  what it costs, how text is delivered into the frontmost application, where the
  user's clipboard and focus end up afterwards, and what happens when the
  permission that gates all of it is missing.
- `templates`: the placeholder vocabulary, how a template is parsed into the
  values it needs, how it renders, what an unresolved or unknown placeholder
  does, and where the caret lands.
- `snippets`: creating, editing, and removing a snippet, finding it in root
  search, and what happens when it is confirmed.
- `quicklinks`: creating, editing, and removing a quicklink, supplying its
  query, and how the query reaches the URL safely.

### Modified Capabilities

None. The form submit path this change builds is already a requirement under
`view-protocol` ("Form collects input"), specified in M1 and never implemented.
Nothing about that requirement changes; it stops being a lie.

## Impact

- **Affected code**: `src-tauri/src/platform/` (selection reading, key
  injection, and focus handling per platform, behind the two new traits),
  `src-tauri/src/templates/` (new), `src-tauri/src/extensions/snippets/` and
  `src-tauri/src/extensions/quicklinks/` (new), `src-tauri/src/invocation.rs`
  and `src-tauri/src/lib.rs` (the form submit command and the values reaching a
  running command), `src/lib/ProtocolView.svelte` and `src/App.svelte` (wiring
  `onsubmit` to something).
- **Database**: a forward-only migration adding snippet and quicklink tables.
  Both are the user's own content and both should exist on the author's two
  machines, so they follow the syncable convention with UUID `id`, `updated_at`,
  and `deleted_at`, unlike the clipboard history.
- **Dependencies**: three new crates, evaluated in design.md against the
  project's off-the-shelf-first requirement: `enigo` for key injection on both
  platforms, `objc2-application-services` for the macOS accessibility API, and
  `minijinja` for template rendering. No new crate for Windows selection
  capture, which is built on the clipboard access this project already has.
- **Permissions**: macOS gains a real Accessibility dependency. Until it is
  granted, both capabilities fail, and failing clearly is part of the spec.
- **Clipboard history**: pasting borrows the clipboard, and the watcher's
  own-write suppression is what keeps that out of the history. Suppression is
  one shot today, and a paste makes two writes: the text going out and the
  user's own content going back. Getting that wrong records what was pasted and
  reorders the entry that was restored, so the watcher changes here.

## Non-goals

- **No hotkeys on snippets or quicklinks.** Binding a command to a global
  shortcut is M5, with a recorder UI and conflict detection it deserves. Here
  they are found by typing in root search, which is the launcher's main surface
  and enough to prove the plumbing.
- **No AI transforms.** M6 is the other consumer of this path and it is not
  pulled forward. The point of building the plumbing now is that M6 finds it
  finished.
- **No window management.** M4 also depends on the Accessibility permission this
  change surfaces, and on nothing else here.
- **No preferences user interface.** Snippets and quicklinks are the user's
  records, created through their own forms, not through the preference schema.
  M7 still owns the preferences window.
- **No rich text, no images, no files.** Templates render plain text. A snippet
  that carries formatting is a second content model in the store and in the
  paste path, and nothing yet needs it.
- **No template control flow.** Placeholders substitute values. Loops and
  conditionals are power a snippet does not need, so they are not specified, not
  documented, and not tested. The design says why they are not blocked either.
- **No sync.** The tables carry the syncable columns from their first migration,
  as every table must. Nothing reads them until M9.
- **No script or shell placeholders.** A placeholder that runs a command is a
  different security question and belongs with M8's script commands.
- **No Linux.** Out of scope for the project.
