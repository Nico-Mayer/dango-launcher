# Proposal

## Why

Design system v2 (`.design/v2/Launcher Component System v3.dc.html`) moves
Dango to herdr's 23 theme roles and its Graphite look. Today `Theme` has 11
ad hoc colour roles with a red accent, and the dark and light values sit inline
in each property. That leaves no clean place for a second theme set. This
change is the first in a stack that brings in v2. The later changes (progress
atoms, task registry, inline answer, component adoption) build on the new roles.

## What Changes

- `Theme` takes herdr's colour role names: canvas, chrome, background,
  surface, popover, raised, hover, selected, selection, border, input,
  foreground, muted, dim, accent, accent-foreground, solid, solid-foreground,
  working, blocked, done, idle, unknown. It keeps `danger` and `shadow`, which
  herdr lacks.
- Two built-in themes fill every role: **Graphite** (dark) and **Paper**
  (light). Dango still follows the operating system's appearance. Dark shows
  Graphite, light shows Paper.
- A theme becomes one struct value inside `theme.slint`, and `Theme` picks
  one. A later custom-theme change only adds one more value to pick.
- The accent changes from red to Graphite's blue grey (`#a7b4c6`).
- **BREAKING (look):** sizes follow the design. List rows grow from 40 to 44
  logical pixels, the SearchBar from 56 to 64, the ActionBar from 40 to 44,
  and the list's padding from 6 to 8. About 8 rows fit before the launcher
  reaches its maximum height, down from 9.
- Toast and HUD dots use the status roles: busy is `working`, success is
  `done`, and failure stays `danger`.
- Tags take a tinted fill in their role in place of an outline.
- Every surface in `ui/` moves to the new roles, the HUD included. The
  launcher takes `popover`, the ActionBar takes `chrome`, and selected rows,
  tiles and actions take `selected`.

## Non-goals

- No theme file, no theme picker, no accent or radius setting. Custom themes
  are a later change.
- The other herdr presets (ink, moss, atom-one-dark) are not shipped.
- No density setting: the row is 44 px.
- Size and font token names stay (`row-height`, `font`, `mono`). Only colour
  roles take herdr names.
- Radius stays 8 px. The monospace face stays Geist Mono.
- No new components and no change to what any component does. Progress
  atoms, skeleton, toast actions, ProgressView and InlineAnswer come in later
  changes.
- The "Nothing animates" requirement stays for now. The progress-atoms change
  removes it.

## Platforms

Windows and macOS. The change is platform-neutral: it touches only `.slint`
files, the height constants in `src/slint_shell/render/`, and tests. Both
platforms already read the appearance through std-widgets' `Palette`, and
that stays.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `design-system`: the two themes get names, and every colour Dango draws
  comes from the active theme. Rows, SearchBar, and ActionBar get the design's
  heights. Selected items use their own `selected` role. Toast dots use the
  status roles. Tags are tinted fills.
- `hud`: the success dot is drawn in the done role, as the toast's is.

## Impact

- `ui/theme.slint`: rewritten with a private preset struct, two presets, and
  role properties.
- Every `.slint` file under `ui/atoms`, `ui/shell`, `ui/views`: role
  renames.
- `src/slint_shell/render/height.rs`: `ROW`, `SEARCH_BAR`, `ACTION_BAR`, and
  `LIST_PADDING` follow `Theme`.
- UI tests in `src/slint_shell/tests.rs` that assume the old sizes and colours.
- `AGENTS.md`: the tech stack note on the single `Theme` global describes
  presets.
