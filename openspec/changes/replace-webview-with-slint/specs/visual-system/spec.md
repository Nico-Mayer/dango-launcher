## MODIFIED Requirements

### Requirement: Shared presentation responds consistently to theme values

Dango SHALL use a consistent presentation for equivalent surfaces and controls
across root search, pushed lists, forms, detail views, and the action panel.
All colours, corner radii, and type sizes SHALL come from one built-in
palette, so changing a value there changes every surface that uses it,
including the action panel.

The built-in dark palette is the only appearance. This capability SHALL NOT
introduce theme file loading, a theme picker, a light palette, or automatic
operating-system theme selection.

#### Scenario: Shared selection appearance changes consistently

- **WHEN** the selection colour in the built-in palette is changed
- **THEN** selected items in root search, pushed lists, and the action panel use
  that value
- **AND** hover, focus, and error colours remain independent values

#### Scenario: Floating content inherits the presentation

- **WHEN** the type size, corner radius, or surface colour in the built-in
  palette is changed and the action panel opens
- **THEN** the panel uses the changed values rather than its own

#### Scenario: No theme configuration exists

- **WHEN** Dango starts without any theme file
- **THEN** the built-in dark appearance is immediately usable
- **AND** no theme file is read or created by this capability

### Requirement: Refinement preserves density and content identity

The default presentation SHALL retain the current search-and-results layout and
density. It SHALL distinguish primary text from supporting text through weight
and contrast, preserve extension tint identities, and preserve the aspect ratio
of application icons and quicklink favicons.

#### Scenario: Default search density is preserved

- **WHEN** root search or a pushed list is displayed at the default text scale
- **THEN** result rows are 56 logical pixels high and the search header is 64
  logical pixels high
- **AND** the footer remains anchored below the scrollable content

#### Scenario: Icons retain their meaning

- **WHEN** results include tinted named icons, application icons, quicklink
  favicons, and clipboard image previews
- **THEN** named icons keep their extension tint, application icons and favicons
  remain untinted and contained, and content previews retain their distinct
  framed treatment

### Requirement: Interaction states remain clear and accessible

Enabled controls SHALL expose visible keyboard focus and distinguish their
default, hover, active, and applicable disabled or invalid states. Keyboard
selection SHALL remain distinct from both pointer hover and keyboard focus,
following the existing list-navigation requirements. A command scope that keeps
focus on its prompt or action list SHALL convey the focused option through its
selected row, not through a ring around the whole surface.

Required text SHALL remain readable rather than relying on faint opacity for
hierarchy. Errors and busy states SHALL include text or accessible state
information instead of relying on colour alone.

#### Scenario: Built-in text and focus contrast

- **WHEN** the built-in dark palette is reviewed
- **THEN** normal-size enabled text, including subtitles and shortcut hints, has
  at least 4.5:1 contrast against its actual background
- **AND** the focus indicator has at least 3:1 contrast against adjacent colours

#### Scenario: Hover cannot impersonate selection

- **WHEN** a different row is hovered while another row is selected
- **THEN** the hovered row has a weaker treatment and the selected row remains
  the clear confirmation target
- **AND** a keyboard press removes the resting hover treatment immediately

#### Scenario: Form error remains associated with its field

- **WHEN** a template field reports an error
- **THEN** its message is visually distinguishable and exposed to assistive
  technology as the description of that field
- **AND** focus and editing remain available

### Requirement: Motion communicates state without delaying work

Dango SHALL NOT animate its interface. Launcher activation, keyboard
selection, hover, the action panel, result changes, and full-tree replacements
SHALL change state in a single frame, without entrance or exit effects.

Ongoing work SHALL be shown by a static busy indicator next to its status
text. No looping effect SHALL run.

#### Scenario: Activation stays immediate on Windows

- **WHEN** the launcher is summoned for its first or a repeat activation on
  Windows
- **THEN** it is painted without visible delay after the global shortcut
- **AND** it does not start invisible or displaced by an entrance effect

#### Scenario: Activation stays immediate on macOS

- **WHEN** the launcher is summoned for its first or a repeat activation on
  macOS
- **THEN** it is painted without visible delay after the global shortcut
- **AND** it does not start invisible or displaced by an entrance effect

#### Scenario: Overlay feedback is short and non-blocking

- **WHEN** the action panel opens or closes
- **THEN** it appears or disappears in one frame, without a transition
- **AND** keyboard navigation in it works at once

#### Scenario: A streamed view stays stable

- **WHEN** a command emits thirty replacements of the same view over one second
- **THEN** content updates without flicker or dropped frames
- **AND** selection and focus are not reset

#### Scenario: Idle and hidden surfaces do not animate

- **WHEN** work is in progress, finishes, or the launcher is hidden
- **THEN** no animation runs on any surface

### Requirement: Presentation fits the launcher viewport

Content and floating controls SHALL fit within the launcher viewport at
supported window sizes and display scaling. Long content SHALL wrap or truncate
appropriately without displacing primary controls or creating horizontal
overflow. Overflowing action lists SHALL scroll while keeping the selected
action visible.

#### Scenario: Compact viewport and long content

- **WHEN** the launcher is shown at 480 by 300 logical pixels with long result
  titles, subtitles, actions, and form help
- **THEN** the footer and active controls remain reachable, floating content
  stays within the window, and nothing scrolls horizontally

#### Scenario: Long action list

- **WHEN** the panel has more actions than fit vertically and the user navigates
  to the last action
- **THEN** the panel scrolls that action into view without moving selection in
  the parent list

#### Scenario: High display scaling

- **WHEN** the launcher is shown at 200 percent display scaling on either
  supported platform
- **THEN** panel corners, icons, text, and focus indicators remain legible and
  are not clipped by the native window edge

## REMOVED Requirements

### Requirement: Reduced motion retains all state information

**Reason**: The interface no longer animates anything (see "Motion
communicates state without delaying work"), so there is no motion to reduce.
Busy, focus, selection, and error information are static already.

**Migration**: None. When a later change brings motion back, it brings back a
reduced-motion requirement with it.
