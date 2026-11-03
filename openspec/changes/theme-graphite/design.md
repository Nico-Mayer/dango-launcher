# Design

## Context

`ui/theme.slint` holds one `Theme` global. Each colour is a property with
inline `dark ? x : y` values, and `dark` comes from std-widgets'
`Palette.color-scheme`. Components read `Theme.<role>` directly (about 160
reads across `ui/`). Rust passes no colours. The renderer counts heights with
constants in `src/slint_shell/render/height.rs` that mirror `Theme` sizes.

The source for the new values is `.design/v2/Launcher Component System
v3.dc.html`: the `THEMES` table (`graphite` and `paper`) for colours, and the
anatomy markup for sizes. The design notes that only Graphite's accent is
checked against herdr. The other values are the design's approximations, and we
take them as our own.

## Goals / Non-Goals

**Goals:**

- One struct that holds a full theme, and two values of it (Graphite, Paper).
- `Theme` exposes herdr's colour role names, so a herdr theme maps one to one
  later.
- Every component moves to the new roles in this change, so no old colour
  name survives.

**Non-Goals:**

- Loading themes from disk, choosing a theme, or changing accent, radius, or
  density at runtime.
- Renaming size or font tokens.
- New components or behaviour (see proposal non-goals).

## Decisions

### A theme is a private struct value inside `Theme`

```slint
struct ThemeColors { canvas: color, chrome: color, /* ...all roles */ }

export global Theme {
    out property <bool> dark: Palette.color-scheme == ColorScheme.dark;
    property <ThemeColors> graphite: { canvas: #0f1012, /* ... */ };
    property <ThemeColors> paper: { canvas: #ecebe7, /* ... */ };
    property <ThemeColors> colors: root.dark ? root.graphite : root.paper;
    out property <color> canvas: root.colors.canvas;
    // one out property per role, so components keep writing Theme.accent
}
```

The struct and the presets are private to `Theme`. Nothing new shows up in the
generated Rust API, apart from the flat role properties that replace today's.
Components keep reading `Theme.<role>`. Struct literals as property defaults,
`#rrggbbaa` literals, and `cond ? struct : struct` all compile in Slint 1.18
(the review checked the vendored compiler).

Custom themes later need a value that does not come from `.slint`. That change
decides where it comes from, and if it is Rust, it amends the AGENTS.md rule
"Rust never passes colour". This change does not decide it.

Alternatives rejected:

- **Rust sets every colour** (`in-out` properties and a `Preset` struct in
  Rust, as the design's `src/theme.rs` sketch does). That breaks the AGENTS.md
  rule for no gain today. It also moves the OS scheme check out of Slint,
  where `Palette` gives it for free.
- **Keep inline `dark ? x : y` per property.** Simple, but a third theme
  would turn every property into a nested ternary. The struct is the
  groundwork the change asks for.
- **Keep our own role names.** That saves the renames, but herdr names make a
  later herdr theme import a straight copy.
- **An exported `Themes` global and a public `colors` property.** Both leak
  the struct into the Rust API for no reader.

### One rename pass, no temporary old names

Two old names are also new herdr names with a different meaning:

- `background` was the window fill, and is now the window fill `popover`.
- `selection` was the selected-row fill, and is now `selected`. `selection`
  stays as the text-selection colour.

So old names can't stay behind as aliases while components move. All colour
reads move in one commit. The Slint compiler rejects any unknown name, so a
missed read fails the build. The risk is a wrong mapping, and the table below
is the guard.

### Role mapping

| Old role | New role | Notes |
|---|---|---|
| `background` (window), `surface` (action panel, dropdown and filter popups) | `popover` | Design: "popover = launcher, menus" |
| - | `chrome` | ActionBar fill |
| `surface` (inputs) | `surface` | Text field, text area, dropdown, checkbox |
| `raised` | `raised` | Keycaps, icon placeholder |
| `border` | `border` | Hairlines, launcher frame, HUD pill |
| - | `input` | Border of text field, text area, dropdown, checkbox |
| `text` | `foreground` | |
| `muted` | `muted` | |
| - | `dim` | Placeholders (SearchBar, form fields) |
| `accent` | `accent` | Red to `#a7b4c6` (Graphite), `#5b6b82` (Paper) |
| `selection` on rows, tiles, actions, filter options | `selected` | |
| `selection` as text selection colour | `selection` | Now accent-tinted |
| `hover` | `hover` | |
| `danger` | `danger` | Not a herdr role. Values from the design: `#ef7a6e` (Graphite), `#c8473b` (Paper) |
| `shadow` | `shadow` | Not a herdr role, not in the design. Today's values: `#00000080` dark, `#00000030` light |
| - | `canvas`, `background`, `accent-foreground`, `solid`, `solid-foreground` | Defined, unused for now |
| - | `working`, `done` | Busy and success dots |
| - | `blocked`, `idle`, `unknown` | Defined, used by the progress atoms change |

Unused roles stay in the struct, so each preset is a full herdr set. That is
the custom-theme groundwork. The spec does not list them.

The CSS `rgba()` values in the design become Slint 8-digit hex. For example,
Graphite `hover` `rgba(255,255,255,.05)` becomes `#ffffff0d`, and `selection`
`rgba(167,180,198,.28)` becomes `#a7b4c647`.

### Toast and HUD dot roles

The design draws busy with `working` and success with `done`. Its failure
example uses `blocked` (amber). We keep `danger` for failure, as the spec does
today: a failed action is not "waiting on you", and red already marks
destructive actions and form errors. The user agreed. The HUD reuses `Dot`, so
its success dot moves to `done` too (hud delta).

### Sizes

Taken from the design's anatomy:

| Token | Old | New |
|---|---|---|
| `row-height` | 40 | 44 |
| `search-height` | 56 | 64 |
| `action-bar-height` | 40 | 44 |
| `list-padding` | 6 | 8 |

`ROW`, `SEARCH_BAR`, `ACTION_BAR`, and `LIST_PADDING` in `render/height.rs`
change in the same commit, per the AGENTS.md rule on fixed heights. The
maximum height stays 480: 64 + 44 + 2 x 8 leaves room for 8 rows.

Kept as they are, by choice rather than from the design: radius 8 px (the
design's default tweak; it draws the window at radius x 2), heading height,
panel row height, and panel padding. Token names stay.

### Tag fill

The design draws a tag as its role's text colour on an 18 % mix of that colour.
In Slint this is `role.with-alpha(0.18)` in `ui/atoms/tag.slint`, not a new
role. The tones are opaque, so `with-alpha` and `transparentize` give the same
result.

### Components

No new component. Every part in `ui/atoms`, `ui/shell`, and `ui/views` keeps
its job and only changes the roles it reads. The HUD pill (`ui/shell/hud.slint`)
moves from `raised` to `popover` with a `border` line, like the launcher frame.
That is our choice: the design has no HUD.

### Contracts

There is no manifest or view-protocol change. Tag roles stay accent, muted, and
danger.

## Risks / Trade-offs

- **The design's hex values are approximations.** If herdr's real files differ
  later, a change swaps the values in one place.
- **The accent loses contrast.** Graphite's `#a7b4c6` is softer than the red.
  The loading sweep, focus borders, and selected-tile outline get quieter. The
  spec keeps the tile outline in the accent role. If it reads too weak in daily
  use, a follow-up change moves it.
- **Fewer results fit:** 8 rows instead of 9 before the window reaches its
  maximum height. The user accepted this.
- **Colour mapping is not tested per element.** Slint's `ElementHandle` test
  API cannot read `background` or `selection-background-color`. Tests check
  `Theme` values. The mapping table and review guard the rest.
