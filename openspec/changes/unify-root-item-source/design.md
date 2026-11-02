# Design

## Context

A root row's right-hand text comes from `Candidate::subtitle`. Commands fill it
from `CommandDecl::subtitle` (`ExtensionHost::command_candidates`), and root
items providers fill it themselves. `Core::result_item` copies it into
`ResultItem::subtitle`, and `render/list.rs::result_row` passes it to the
`subtitle` text of the list row in `ui/views/list.slint`.

`Core` already has every extension's manifest name: `extension_contexts` builds
a `ui::Context { title, icon }` per extension at startup, and `result_item`
puts it on each `ResultItem` for the ActionBar.

## Goals / Non-Goals

**Goals:**

- One place decides a root row's right-hand text.
- An extension cannot accidentally put a path or preview there.

**Non-Goals:**

- Changing the list row component or its layout. The same `subtitle` text in
  `ui/views/list.slint` shows the new value. No new design-system part.
- Changing `ListItem::subtitle` in the view protocol. Pushed views keep it.

## Decisions

### `Core::result_item` picks the text

`result_item` sets `ResultItem::subtitle` to the candidate's status line for a
pinned candidate and to `context.title` for every other one. The renderer and
the state machine stay as they are: they still show whatever `subtitle` holds.

Alternatives:

- *Renderer reads `context.title`.* Rejected: the renderer holds no rules
  (AGENTS.md, Architecture).
- *Snapshot builder in `src/ui/` decides.* It would work, but then
  `ResultItem` would carry both a status line and a context only for that
  choice. `Core` already builds the item and has the name, so the choice is one
  line there.
- *Each extension sets its own name as subtitle.* Rejected: that is how the
  column drifted in the first place.

### `Candidate::subtitle` becomes `Candidate::status`

The field is renamed so its name says what it is for: a status line that only
pinned items set (Permissions, Updates). `result_item` ignores it on a
non-pinned candidate.

Alternatives:

- *Keep the name `subtitle` and ignore it when not pinned.* Rejected: a field
  called subtitle that is silently dropped invites the next provider to set it.
- *Fold it into `pinned: Option<String>`.* Rejected: pinning and having a
  status line are separate facts, and the ranker reads `pinned` on its own.

### Remove `CommandDecl::subtitle`

With the name coming from the manifest, a per-command subtitle has no use.
**This changes the extension manifest, a versioned contract.** Dango is alpha,
so the field is removed outright: no compat, and `manifest_version` stays 1.
Third-party extensions do not exist yet, so no manifest in use breaks.

Alternative: keep it as an override. Rejected: an override is exactly what
brought back "Dango" and "Clipboard".

## Interface copy

No new strings. The changed right-hand texts are existing extension names,
already Title Case per `interface-copy`:

- "Applications", "System", "Games", "Updates", "Clipboard History",
  "Snippets", "Quicklinks"

Pinned status lines ("You have 0.4.0", the permission lines) are unchanged.

## Risks / Trade-offs

- [Two apps with the same name, such as two Firefox installs, can no longer be
  told apart by path in the row] → Accepted for now; the icon often differs.
  Showing the path elsewhere is a separate change (proposal, Non-goals).
- [Snippets with similar names lose their body preview in root] → Accepted by
  the author; "Search Snippets" still shows it.
