## Why

The launcher's look grew one surface at a time: a 15-colour palette with
extension tints, Fluent widgets that cannot take our colours, a failure banner,
and one 825-line `.slint` file. The author has designed a complete component
system for the launcher (`.design/Launcher Component System.dc.html`). It
replaces the old look and its rules completely, and gives commands richer
views than a plain list, detail, and form.

## What Changes

- **BREAKING** A new design system replaces the current visual rules: a `Theme`
  global with nine colour roles plus hover, one radius, one row height, one gap,
  and three type sizes, set in Geist and Geist Mono, in a dark and a light
  theme that follow the operating system.
- The launcher is rebuilt from the design's parts: LauncherWindow, SearchBar,
  ActionBar, the view stack, List, Grid, Detail with Metadata, Form,
  ActionPanel, Toast, EmptyView, and the atoms Icon, Kbd, Tag, and Accessory.
- The window keeps its width and grows with its content up to a maximum
  height, instead of a fixed height.
- The SearchBar shows a back control when a view is pushed, a loading bar while
  the view is busy, and an optional filter dropdown declared by the view.
- The ActionBar replaces the footer. It shows the current command's name and
  icon, or a toast, and the primary action with its key and the action panel
  key.
- **BREAKING** Failures move from the banner into a toast in the ActionBar and
  are one short sentence. Detail goes to the log.
- The action panel gains sections, a `danger` style for destructive actions,
  and its own filter input.
- **BREAKING** View protocol version 2: list items gain sections, accessories,
  and keywords; a list can show a detail pane for its selected item; a detail
  view gains metadata; a new grid view kind; actions gain a section and a
  style; views can declare a filter dropdown and a success toast. Version 1 is
  no longer accepted.
- **BREAKING** Extension tints are removed, from the manifest and from root
  search.
- **BREAKING** The development build marker is removed: no frame colour, no
  "dev" badge, no tray line.
- The "brief confirmation" after a no-view command is removed from the spec.
  Nothing implements it today; a HUD comes in its own change.
- `AGENTS.md` and `openspec/config.yaml` are rewritten where they describe the
  old UI: the Fluent style, std-widgets first, the `Palette` global, and the
  single `ui/app.slint`.

Data flow stays as it is: the pure reducer in `src/ui/`, the snapshot, the
Effect and Event loop, the `ViewTree` protocol, and the `Shell` trait. Only the
layer between the renderer and Slint changes.

Platforms: Windows and macOS, both fully covered. The work is platform neutral
except the window's height change and the operating system theme switch, which
get a manual check on each platform.

## Non-goals

- A HUD after the launcher hides.
- A theme file, a theme picker, a custom accent, or a density setting.
- Animation of any kind, including the design's animated toast.
- The design's "Rust binding" section: a `Command` trait that returns a view per
  keystroke, an `Outcome` enum, and debouncing input.
- Rendering full markdown beyond what Slint's styled text supports.
- New built-in commands that use the grid or the filter dropdown. This change
  ships the components; built-ins adopt them later.
- Third-party extensions or the sandboxed host.

## Capabilities

### New Capabilities

- `design-system`: the theme roles and how they apply, light and dark themes
  following the operating system, the type faces, density, the shell
  regions and their sizes, the atoms, the toast, focus and hover treatment,
  overlays, no motion, and fitting the viewport.

### Modified Capabilities

- `visual-system`: removed, replaced by `design-system`.
- `development-indicator`: removed.
- `view-protocol`: version 2 with sections, accessories, keywords, detail pane,
  metadata, grid, action sections and styles, filter dropdown, and success
  toast; the footer becomes the ActionBar; no-view success no longer promises
  a confirmation.
- `root-search`: the tint requirement is removed.
- `extension-model`: the tint requirement is removed.
- `interface-copy`: failures are one sentence in the toast instead of the
  banner; footer wording becomes ActionBar.
- `launcher-shell`: the window's height follows its content up to a maximum,
  with its top edge fixed.
- `list-navigation`: grid navigation with all four arrows, and the action
  panel's filter input owning typing while the panel is open.

## Impact

- `ui/app.slint` is replaced by `ui/theme.slint`, `ui/shell/`, `ui/views/`, and
  `ui/atoms/`, plus `ui/fonts/` with the Geist and Geist Mono files (SIL Open
  Font License).
- `src/slint_shell/render.rs` is split per view and writes Slint globals instead
  of window properties. `src/slint_shell/tests.rs` follows.
- `src/ui/`: snapshot types and reducer rules for sections, accessories, grid,
  the detail pane, the panel filter, the filter dropdown, and the toast.
- `src/protocol.rs`: protocol version 2.
- Built-in extensions in `src/extensions/`: tints removed, views moved to
  version 2.
- `src/extension/manifest.rs` and the manifest schema: the `tint` field goes.
- Window sizing in `src/slint_shell/` and the platform positioning code, so the
  top edge stays fixed as the height changes.
- `AGENTS.md`, `openspec/config.yaml`.
- No change to the config file or stored data.
