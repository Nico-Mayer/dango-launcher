# Proposal

## Why

The text on the right of a root search row means something different for each
extension. Applications show a file path, most commands show their
extension's name in a short or different form ("Clipboard", "Dango"),
settings panes show nothing, Steam games show "Steam", and snippets and
quicklinks show their body. You cannot read the column at a glance, because
there is no rule for what it contains.

## What Changes

- Every root search row shows the name of the extension that owns it on the
  right, taken from that extension's manifest. Extensions no longer choose
  this text per command or per item.
- Pinned rows (missing permission, update available) keep their own status
  line, because it tells the user something they need to act on.
- **BREAKING**: remove `subtitle` from the command declaration in the
  extension manifest. No migration, because Dango is alpha.
- Visible results for each built-in:
  - Applications: "Applications" (was the app's path)
  - Settings panes: "System" (was empty)
  - Steam games: "Games" (was "Steam")
  - Check for Updates: "Updates" (was "Dango")
  - Clipboard History: "Clipboard History" (was "Clipboard")
  - Snippets in root: "Snippets" (was the first line of the body)
  - Quicklinks in root: "Quicklinks" (was the URL)
  - System, Window Management, Games, and AI commands: unchanged

## Non-goals

- Showing the app path somewhere else (detail pane, tooltip, new action). If
  two apps with the same name become hard to tell apart, that is its own
  change.
- Subtitles in pushed views. "Search Snippets" and other view-protocol lists
  keep their item subtitles, such as the snippet body preview.
- Accessories, tags, or a second column on root rows.
- Renaming any extension.

## Platforms

Windows and macOS alike. The change is in shared search, Core, and built-in
extension code. No platform code changes, so there is nothing to check by
hand on either platform.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `root-search`: new requirement that each result names its owning extension,
  with pinned rows showing their status line instead.
- `extension-model`: a command declaration no longer has a subtitle.

## Impact

- `src/extension/manifest.rs`: `CommandDecl::subtitle` removed; every built-in
  manifest and test fixture drops the field.
- `src/search/mod.rs`: `Candidate::subtitle` becomes a status line that only
  pinned items set.
- `src/app/mod.rs`: `Core::result_item` chooses the row's subtitle.
- `src/extensions/applications`, `games`, `snippets`, `system`: root items stop
  setting a subtitle.
- `src/extensions/permissions`, `updates`: pinned items move to the status
  field.
- Tests in `src/ui/tests.rs`, `src/slint_shell/tests.rs`, `src/app/tests.rs`,
  and in each extension module that assert on subtitles.
