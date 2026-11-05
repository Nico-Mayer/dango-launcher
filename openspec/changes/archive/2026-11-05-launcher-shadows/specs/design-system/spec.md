# Spec Delta

## MODIFIED Requirements

### Requirement: Every screen is one shell around one view

The launcher SHALL show, from top to bottom, the SearchBar, exactly one view,
and the ActionBar. The view is a list, a grid, a detail, a form, a progress
view, or an empty view. In root search, an answer card MAY sit between the
SearchBar and the results; when no result matches, the card alone stands in
for the empty view. Overlays SHALL float above these regions and SHALL NOT
move them.

The SearchBar SHALL be a pill on its own. The view and the ActionBar SHALL sit
together in one card below it, separated from the pill by a gap through which
what is behind the launcher shows. At root with an empty query the card MAY be
hidden, as root-search describes; the pill alone is then the launcher.

The pill and the card SHALL each be 720 logical pixels wide. The pill SHALL be
52 logical pixels high and fully rounded at its ends, the gap 8, the ActionBar
44, and a list row 44.

The pill and the card SHALL each cast the same shadow as the action panel, in
the theme's shadow colour, drawn in full and not cut off by the window edge.
Where the platform falls back to software rendering (Windows without OpenGL
2), no shadow SHALL be drawn and nothing else SHALL change: that renderer
cannot draw shadows.

#### Scenario: Pill and card cast a shadow

- **WHEN** root search shows the pill and the card on a machine with GPU
  rendering, on Windows or macOS
- **THEN** each casts a soft shadow, offset downwards, that fades out before
  the window edge

#### Scenario: No shadow under software rendering on Windows

- **WHEN** the launcher is shown on Windows without OpenGL 2
- **THEN** the pill and the card are drawn without a shadow, at the same size
  and place

#### Scenario: Root search

- **WHEN** the user types a query at root
- **THEN** the launcher shows the SearchBar pill and, below the gap, one card
  holding a list of results and the ActionBar

#### Scenario: Sizes at default text scale

- **WHEN** root search shows results at the default text scale
- **THEN** the pill and the card are 720 logical pixels wide, the pill is 52
  high, the gap 8, each result row 44, and the ActionBar 44

#### Scenario: The gap shows what is behind

- **WHEN** the launcher shows the pill and the card over another application
- **THEN** that application is visible through the gap between them, shaded
  only by the pill's shadow

#### Scenario: Root search with an answer

- **WHEN** the user types "2+2" at root
- **THEN** the card shows the answer card, the results, and the ActionBar, from
  top to bottom

#### Scenario: A pushed view

- **WHEN** a command pushes a list, grid, detail, or form
- **THEN** the pill and the card are both shown
