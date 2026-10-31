## Context

See `proposal.md` for why. The source design is
`.design/Launcher Component System.dc.html`. Requirements are in this change's
specs, mainly `design-system` and `view-protocol`.

Today the launcher is one component, `LauncherWindow` in `ui/app.slint`, with
about 20 `in` properties fed by `src/slint_shell/render.rs`. The renderer turns
each `Snapshot` from the pure reducer in `src/ui/` into flat models (`RowData`
with heading rows, `ActionData`, `FieldData`). Inputs are Fluent std-widgets.

Facts checked for this design:

- The Fluent std-widgets `Palette` exposes every colour as `out`; only
  `color-scheme` is `in-out` (`i-slint-compiler-1.18.1/widgets/fluent/style-base.slint`).
  Fluent `LineEdit`, `TextEdit`, `CheckBox`, `ComboBox`, and `Button` cannot take
  our colours, radii, or focus border.
- The same `Palette.color-scheme` reads the operating system's scheme when
  left unset (`fluent/styling.slint`).
- winit 0.30.13 resizes a macOS window with `setContentSize`
  (`platform_impl/macos/window_delegate.rs`), which keeps the bottom-left
  origin, so the window grows upward. Windows keeps the top-left.
- Slint embeds a font with `import "./file.ttf";` in a `.slint` file and falls
  back to system faces for missing glyphs.

## Goals / Non-Goals

**Goals:**

- The `.slint` side is split into small files that map one to one to the
  design's parts.
- Every value comes from `Theme`; nothing else holds a colour or size.
- Every new interaction rule (grid arrows, panel filter, filter dropdown, toast
  lifetime, detail pane) lives in the reducer and is tested with `cargo test`.

**Non-Goals:**

- No change to the Effect and Event loop, `Core`, `Shell` beyond one resize
  call, the extension host, or search.
- No new built-in commands. Built-ins move to protocol version 2 with what they
  show today, and clipboard history gains its "All types" / "Text only" filter
  only if a later change asks for it.

## Decisions

### D1. Keep the data flow, replace the binding layer

The reducer, `Snapshot`, Effect and Event, `ViewTree`, and `Shell` stay. The
design's "Rust binding" section is not adopted.

Rejected: the design's `Command` trait with `view(&mut self, query) -> View`
and `Outcome`. It runs on the UI thread per keystroke, which breaks
"search never blocks" and cancel-per-keystroke; it cannot stream (AI
commands); `Push(Box<dyn Command>)` cannot cross the future sandbox boundary;
and it would move interaction rules out of the tested reducer. Our
`ActionOutcome` already covers its outcomes; it gains only `Toast(String)`.

### D2. Slint globals instead of window properties

The renderer writes exported globals, one per area:

```
Theme    colours, radius, sizes, fonts         (ui/theme.slint, pure Slint)
Shell    context, toast, primary label, keys, back, loading, height
Search   query revision, placeholder, filter title
ListData rows (flat, headings inline), detail pane, empty view
GridData tiles, columns, aspect
Panel    open, filter text, rows (headings inline), selected
FormData fields
Intents  callbacks into Rust (key, query, field, activate, filter, panel)
```

Each component file imports only the globals it reads, so no property is
threaded through `LauncherWindow`. Rows stay flat with inline headings, as
today, so `reveal` keeps computing offsets from fixed heights.

Rejected: nested models (`[Section { items: [Item] }]`). Slint can hold them,
but flat rows keep scrolling and selection maths in one place.

### D3. File layout

```
ui/
  theme.slint             Theme global, dark and light values
  fonts/                  Geist 400/500/600, Geist Mono 400/500 (OFL)
  atoms/   icon, kbd, tag, accessory, dot, text-field, text-area,
           checkbox, dropdown, separator
  shell/   launcher-window, search-bar, action-bar, toast, action-panel,
           filter-popup
  views/   list, grid, detail, metadata, form, empty-view
  app.slint               exports LauncherWindow, Tray, and the globals
```

`src/slint_shell/render.rs` becomes `render/` with one writer per global.

### D4. Inputs are built on `TextInput`, not Fluent widgets

Text field, text area, checkbox, and dropdown are our own components on the
`TextInput`, `Rectangle`, `TouchArea`, and `PopupWindow` primitives. They carry
`accessible-role`, `accessible-label`, `accessible-value`, and
`accessible-description` (errors) so UI tests and screen readers find them.
`ScrollView` stays from std-widgets.

Rejected:

- Fluent widgets: colours are `out` (see Context), so the accent focus border,
  our radii, and both themes cannot be applied.
- Another built-in style (Material, Cupertino, Cosmic): same `Palette`
  restriction, and a foreign look on both platforms.
- Patching std-widgets: Slint ships them inside the compiler; a fork would
  pin us to a patched compiler.

`TextInput` is the primitive `LineEdit` is built on, so editing, selection,
clipboard, and German dead keys behave the same.

### D5. Theme values

`Theme` derives `dark` from std-widgets `Palette.color-scheme`, left unset so
it follows the OS. Values are the design's, adjusted for contrast (checked with
WCAG relative luminance against the blended selection and hover colours):

| Role      | Dark                      | Light                     |
|-----------|---------------------------|---------------------------|
| background| `#161618`                 | `#fbfbfa`                 |
| surface   | `#1e1e21`                 | `#ffffff`                 |
| raised    | `#26262a`                 | `#f1f1ef`                 |
| border    | `#2f2f34`                 | `#e4e4e1`                 |
| text      | `#ededef`                 | `#18181b`                 |
| muted     | `#9a9aa2` (was `#8b8b93`) | `#6b6b72`                 |
| accent    | `#ff6363`                 | `#c0282d` (was `#ff6363`) |
| selection | `#ffffff12`               | `#0000000e`               |
| hover     | `#ffffff08` (new)         | `#00000006` (new)         |
| danger    | `#ff8080` (was `#ef5350`) | `#b71c1c` (was `#d32f2f`) |

Why the changes: dark muted was 4.00:1 on a selected row and 4.46:1 on the
ActionBar; light accent `#ff6363` was 2.91:1 on white; dark danger was 3.88:1
on a selected row. Every role now passes 4.5:1 on every background it meets.

Sizes: radius 8, row 40, gap 8, SearchBar 56, ActionBar 40, icon 20, window
width 720, max height 480; type 12, 14, 16.

### D6. View protocol version 2

`src/protocol.rs` moves to version 2 and rejects 1. Additions, all optional
fields except the new view kind: `ListItem.section`, `accessories`,
`keywords`, `detail`; `ListView.sections` (title, subtitle), `detail_pane`,
`filter`; `GridView` (columns 4 to 8, aspect, items with image, colour, or
glyph content); `DetailView.metadata`; `Action.section`, `style`;
`EmptyState.icon`, `actions`; form field kinds `textarea`, `dropdown`, and a
`separator` entry; `ActionOutcome::Toast`.

This locks the version 2 contract for the future sandboxed host. Item sections
are a field on the item rather than a nested list so a full-tree replace stays
a flat list that keeps selection by identifier.

### D7. Toast lifetime goes through the reducer

The reducer holds the toast. A success toast emits `Effect::ExpireToast { id }`;
`Core` waits about two seconds on tokio and returns `Event::ToastExpired { id }`.
A newer toast changes the id, so a stale expiry does nothing. Any key or click
clears a failure toast. Busy toasts come from the loading and running states.
Failures today held in `snapshot.failure` and shown in the banner become
failure toasts; the banner is removed.

### D8. Window height

The renderer computes the content height from the fixed row, heading, and tile
heights and sends it through a new `Shell::set_launcher_height(f32)`, which
calls a new platform method that resizes keeping the top edge:

- Windows: `SetWindowPos` with `SWP_NOMOVE`, size in physical pixels.
- macOS: `setFrame:display:` with the origin moved down by the change.

`position_on_active_display` centres using the maximum height, so a short
launcher sits where a full one would and the SearchBar never jumps. The height
is set before the snapshot's models are written, in the same main-thread turn.

Rejected:

- Slint `Window::set_size`: on macOS it goes through `setContentSize` and grows
  upward.
- A fixed 480 window with a transparent lower part: on Windows the transparent
  part still takes clicks, so a click just below the launcher would dismiss it
  but not reach the app underneath.

### D9. Grid

Tiles are placed with computed `x` and `y` from index and columns in a
`ScrollView`, not a `GridLayout`, so a `for` repeater can fill rows. Navigation
is reducer logic: Left and Right move by one, Up and Down by `columns`, with
Down clamped to the last tile. While a grid is shown, its Left and Right
bindings are in the snapshot, so the root `FocusScope` takes them before the
SearchBar's `TextInput`.

### D10. Action panel filter and the filter dropdown

The panel gets a `TextInput`. `Focus::Panel` puts the cursor in it; the
reducer owns the filter text and narrows actions by fuzzy title match with the
existing matcher. The view's filter options are appended to the panel as a
"Filter" section (see `view-protocol`). The dropdown in the SearchBar is a
`PopupWindow` listing the same options; choosing one emits
`Intent::FilterChosen`, which the reducer turns into an effect that calls the
command.

### D11. Things that go

`Palette`, `IconTile` tints, `ResultItem.tint`, `Manifest.tint`,
`ExtensionHost::tints`, `tint_index`, `Snapshot.development`, the dev badge and
frame colour, the tray's development tooltip, the failure banner, the error
screen kind (an unsupported view becomes an empty view), and the "Dango" label
in the footer.

### D12. Strings

Written with the ux-writing skill and checked against `interface-copy`. New or
changed:

| Where                         | String                                         |
|-------------------------------|------------------------------------------------|
| Panel filter placeholder      | "Search actions"                               |
| Panel, nothing matches        | "No matching actions"                          |
| Panel section for filters     | "Filter"                                       |
| Back control label            | "Go back"                                      |
| Filter dropdown label         | "Filter results"                               |
| Unsupported view, title       | "This view needs a newer version of Dango"     |
| Unsupported view, hint        | "Press Escape to go back"                      |
| Invalid view, title           | "This view can't be shown"                     |
| Invalid view, hint            | "Press Escape to go back"                      |
| Command gone (toast)          | "That command is no longer available"          |
| Elevated window (toast)       | "Dango can't move administrator windows"       |
| macOS window (toast)          | "{App} doesn't let Dango move its windows"     |
| Accessibility (toast)         | "Allow Accessibility in System Settings"       |
| System refused (toast)        | "The system didn't empty the trash", "…lock the screen", "…go to sleep" |
| App gone (toast)              | "That app is no longer installed"              |

Unchanged and kept: "Search apps and commands", "Search", "Actions",
"Loading…", "Running {title}", "No results for “{query}”", "Nothing to show".
Other existing failure strings in `src/` are cut to one sentence that fits the
toast when they are moved; each one cut is listed in its task's commit.

## Risks / Trade-offs

- [Own inputs miss platform behaviour, such as a text context menu] → Built on
  `TextInput`, which supplies editing and clipboard; check Windows and macOS
  shortcuts (word jump, select all) in the UI tests.
- [Geist lacks a glyph such as ⌘ or ↵] → Slint falls back to a system face;
  a UI test renders every Kbd label, and a missing glyph gets a named icon.
- [Resizing on every keystroke flickers] → Height and models change in one
  main-thread turn; manual check per platform. If it still flickers, round the
  height to whole rows and resize only when it changes.
- [Accent and danger are both red in the dark theme] → Danger actions also sit
  last, under their section, and carry a verb like "Delete"; colour is never the
  only signal.
- [Protocol version 2 is a large contract change] → Alpha with no third-party
  extensions; all built-ins move in this change.
- [Removing the dev marker makes debug and release builds look alike] → Accepted
  by the author.

## Migration Plan

None. Built-ins move to protocol version 2 in the same change, no stored data
changes, and the manifest's `tint` field is deleted from built-ins.
