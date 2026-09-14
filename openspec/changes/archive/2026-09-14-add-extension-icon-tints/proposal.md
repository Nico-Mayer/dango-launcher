## Why

Built-in commands all look the same in root search: a grey glyph on the window
background, four of them in a row. Applications stand out because they carry
their own colourful icons, so the commands between them read as one undifferentiated
block, and telling a window-management command from a system one takes reading the
title rather than glancing at it.

Milestone: M7 polish. It is a visual change to the root list, not one of the
three items M7 lists, and it earns its place there because it costs little and
is felt on every activation.

## What Changes

- The extension manifest gains an optional `tint`: a named colour from a closed
  palette the launcher owns, declared once per extension. Absent or unknown
  means no tint, exactly as today.
- A root search result whose icon is one of the launcher's named icons is drawn
  on a filled rounded square in its extension's tint, with the glyph in the
  tint's paired foreground colour. A result whose icon is an image file, such
  as an application's own icon, is unchanged.
- The palette is defined once in `src/app.css` as light and dark token pairs,
  and named in one frontend registry, in the same spirit as the icon registry.
- Every built-in that contributes named icons declares a tint: `dango.ai`,
  `dango.clipboard`, `dango.snippets`, `dango.quicklinks`,
  `dango.window-management`, and `dango.system`. `dango.applications` declares
  none, because its results already carry real application icons.

Not a breaking change: `tint` is optional, so a manifest written before it still
loads and renders as it does now.

## Non-goals

- No per-command tint. The tint identifies the extension; a command overriding
  it would defeat the point and widen the contract for nothing.
- No tint inside pushed views. Every row in a command's own view comes from the
  same extension, so colour there separates nothing. The view protocol does not
  change.
- No user-configurable tints in `config.json`, and no tint for third-party
  extensions beyond the declared field the contract already gives them.
- No change to icons themselves, to ranking, or to the result row's layout,
  size, or spacing.
- No tint behind the placeholder square shown when an icon is genuinely unknown.

## Capabilities

### Modified Capabilities

- `extension-model`: the manifest may declare a tint for the extension, from a
  palette the launcher owns; an unknown or absent tint renders untinted.
- `root-search`: a result's named icon is presented on its contributing
  extension's tint; a file icon is presented as it is today.

## Platforms

Windows and macOS, identically. This is webview rendering with no platform code
behind it, so there is no platform split to state.

## Impact

- `src-tauri/src/extension/manifest.rs`: optional `tint` field on `Manifest`.
- The six built-in manifests that declare named icons.
- `src-tauri/src/lib.rs`: the search result DTO carries the contributing
  extension's tint, resolved from the host's manifests.
- `src/app.css`: palette tokens. `src/lib/ResultRow.svelte` and a small tint
  registry beside it: rendering.
- No database, config file, or view protocol change.
