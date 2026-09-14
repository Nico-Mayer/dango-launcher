## Context

See proposal.md - Why.

What the current code gives us:

- A named icon is `icon:<name>`; anything else is a path to an image file
  (`src-tauri/src/extension/manifest.rs`, `src/lib/Icon.svelte`).
- `ResultRow.svelte` already branches three ways: named icon, image file,
  placeholder square. Only the first branch is in scope.
- Search results reach the frontend as `ResultItem` in `src-tauri/src/lib.rs`,
  built from `Candidate` in the `search` command's streaming loop. Every
  candidate already carries the `extension_id` that contributed it.
- The `ExtensionHost` is managed behind an `Arc<Mutex<..>>`, because resolving a
  command has to see live enabled state.
- `index.html` hardcodes `class="dark"`, so the light token set in
  `src/app.css` exists but is not exercised today.

## Goals / Non-Goals

**Goals:**

- One tint per extension, decided in one place, so two results from the same
  extension cannot disagree.
- An unknown or absent tint degrades to today's rendering, never to a failure.
- No new cost on the search path.

**Non-Goals:**

- See proposal.md - Non-goals. At design level, also: no change to the view
  protocol, no new dependency, and no change to how icons themselves are
  resolved or cached.

## Decisions

### The tint is resolved from the extension, not carried by each candidate

`ResultItem` gains a `tint` field, filled by looking the candidate's
`extension_id` up in a map built once from the loaded manifests. Built-in
providers are left untouched.

Rejected: adding `tint` to `search::Candidate` and filling it in
`ExtensionHost::command_candidates` plus every root provider. It spreads a
per-extension fact across every producer, which is exactly how two rows from one
extension end up different colours, and it would touch the applications and
snippets providers for no gain.

The map is its own managed state (`extension id -> tint name`), not a read
through the host's mutex. The streaming loop emits a snapshot per provider
answer, and putting the host's lock on that path to fetch a value that cannot
change at runtime is not worth it. Enabling or disabling an extension changes
which results appear, never what colour an extension is, so the map is built
once at startup alongside the rest and never rebuilt.

### `tint` is an optional free string in the manifest, not a Rust enum

This locks in a manifest change, so it is stated plainly: manifest v1 gains an
optional `tint: Option<String>`. Adding an optional field is backwards
compatible - serde defaults it to `None`, existing manifests load unchanged, and
`SUPPORTED_MANIFEST_VERSION` stays at 1. A third-party manifest will be able to
declare it with no further work. Command-level tint is deliberately not part of
the contract; adding it later is additive if it is ever wanted.

The palette is a presentation concern the frontend owns, exactly as the icon
registry is. Rust passes the name through the same way it passes `icon:<name>`
without knowing what a `moon` looks like.

Rejected: a Rust enum of tint names. It duplicates the palette on both sides of
the boundary anyway, and it turns a typo into a deserialization failure for a
whole extension rather than a row that is merely uncoloured.

The cost is that a typo is silent. Mitigated by a Rust test that asserts every
built-in manifest declares a tint from a listed set of names, so the two lists
are kept honest by the one that runs in CI.

### The palette is six token pairs derived from the existing accent pair

`src/app.css` already carries `--accent: hsl(204 94% 94%)` with
`--accent-foreground: hsl(204 80% 16%)` in light, and the two swapped in dark.
Every tint uses that same formula at a different hue, so the palette is the
project's existing token set widened rather than a set of ad-hoc colours, which
is what `openspec/config.yaml` asks for:

| Tint     | Hue | Extension                 |
| -------- | --- | ------------------------- |
| `red`    | 0   | `dango.system`            |
| `amber`  | 45  | `dango.clipboard`         |
| `green`  | 142 | `dango.snippets`          |
| `blue`   | 204 | `dango.window-management` |
| `purple` | 270 | `dango.ai`                |
| `pink`   | 330 | `dango.quicklinks`        |

`dango.applications` declares no tint: its results carry real application icons,
and a coloured square behind one would fight it.

Six hues about 60 degrees apart is the point at which they stay separable at
32px. Adding a seventh later is one line in `app.css`, one in the frontend
registry, and one in the test's list.

Rejected: deriving a colour by hashing the extension id. It needs no
declaration, but it gives no control over which extension gets red, changes
colours when an id changes, and can put two neighbours on the same hue.

### The frontend maps a tint name to literal class strings

A small registry beside `Icon.svelte` maps each tint name to a fixed
`"bg-tint-<name> text-tint-<name>-foreground"` string, with an unknown name
returning nothing.

Tailwind v4 scans source text for class names, so a class assembled at runtime
from a variable is never generated. Writing the six pairs out as literals is
what makes them exist in the stylesheet. The alternative, an inline
`style="background-color: var(--tint-red)"`, would work but steps outside the
token-and-class convention the rest of the app follows for no benefit.

### Rendering and primitives

Only the named-icon branch of `ResultRow.svelte` changes: the existing
`size-8` box gains `rounded-md` and the tint pair, and the icon inside it stays
at 20px. The image and placeholder branches are untouched, so row height,
spacing, and alignment do not move.

No Bits UI primitive is added or patched. The root list stays `Command.Root`,
`Command.List`, `Command.Viewport`, and `Command.Item`; `ResultRow` is a plain
presentational component rendered inside `Command.Item`, and nothing about
selection, keyboard handling, or scrolling is involved.

`ProtocolView.svelte` renders the same `ResultRow` for a pushed list view. It
passes no tint, so those rows keep today's appearance.

### User-facing text

This change adds and edits no user-visible string. There is nothing to check
against `openspec/specs/interface-copy/spec.md`.

## Risks / Trade-offs

- A wall of colour in the root list, which is the failure mode the change is
  trying to avoid → the tint is a 32px square, only behind named icons, only in
  the root list, from a palette of six.
- Contrast in the dark theme, where a 16% lightness background carries a 94%
  lightness glyph → the pair is the one the existing `accent` tokens already
  use, and verification includes reading the list on both platforms. The light
  set is written to the same formula, but it cannot be verified from the running
  app while `index.html` hardcodes `class="dark"`; that is stated rather than
  claimed as tested.
- Two lists of palette names, one in CSS plus the frontend registry and one in
  the Rust test → both are six short lines, and the test fails in CI when they
  drift.
- A tint name that is valid but wrong reads as a colour nobody chose → the
  assignment table above is the record of what was chosen and why.
