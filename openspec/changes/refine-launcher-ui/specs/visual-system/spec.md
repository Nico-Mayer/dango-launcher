## Purpose

Define Dango's cohesive visual presentation and interaction feedback so the launcher stays clear, responsive, accessible, and ready for future theme customization without changing its familiar layout.

## ADDED Requirements

### Requirement: Shared presentation responds consistently to theme values

Dango SHALL use a consistent presentation for equivalent surfaces and controls across root search, pushed lists, forms, detail views, and the action panel. Changing a shared presentation value SHALL affect all consumers of that value, including floating content, without requiring feature-specific styling changes.

The built-in appearance SHALL remain available without external theme files. This capability SHALL NOT introduce theme file loading, a theme picker, or automatic operating-system theme selection.

#### Scenario: Shared selection appearance changes consistently

- **WHEN** the shared selection color is overridden in a development preview
- **THEN** selected items in root search, pushed lists, and the action panel use that value
- **AND** hover, focus, and error colors remain independently adjustable

#### Scenario: Floating content inherits the presentation

- **WHEN** shared typography, corner, and surface values are overridden and the action panel opens
- **THEN** the panel uses the current shared values rather than an unrelated built-in appearance

#### Scenario: No theme configuration exists

- **WHEN** Dango starts without any theme file
- **THEN** the built-in dark appearance is immediately usable
- **AND** no theme file is read or created by this capability

### Requirement: Refinement preserves density and content identity

The default presentation SHALL retain the current search-and-results layout and density. It SHALL distinguish primary text from supporting text through weight and contrast, preserve extension tint identities, and preserve the aspect ratio of application icons and quicklink favicons.

#### Scenario: Default search density is preserved

- **WHEN** root search or a pushed list is displayed at the default text scale
- **THEN** result rows are 56 CSS pixels high and the search header is 64 CSS pixels high
- **AND** the footer remains anchored below the scrollable content

#### Scenario: Icons retain their meaning

- **WHEN** results include tinted named icons, application icons, quicklink favicons, and clipboard image previews
- **THEN** named icons keep their extension tint, application icons and favicons remain untinted and contained, and content previews retain their distinct framed treatment

### Requirement: Interaction states remain clear and accessible

Enabled controls SHALL expose visible keyboard focus and distinguish their default, hover, active, and applicable disabled or invalid states. Keyboard selection SHALL remain distinct from both pointer hover and keyboard focus, following the existing list-navigation requirements.

Required text SHALL remain readable rather than relying on faint opacity for hierarchy. Errors and busy states SHALL include text or accessible state information instead of relying on color or animation alone.

#### Scenario: Built-in text and focus contrast

- **WHEN** the built-in dark or light palette is reviewed in a development preview
- **THEN** normal-size enabled text, including subtitles and shortcut hints, has at least 4.5:1 contrast against its actual background
- **AND** the focus indicator has at least 3:1 contrast against adjacent colors

#### Scenario: Hover cannot impersonate selection

- **WHEN** a different row is hovered while another row is selected
- **THEN** the hovered row has a weaker treatment and the selected row remains the clear confirmation target
- **AND** a keyboard press removes the resting hover treatment immediately

#### Scenario: Form error remains associated with its field

- **WHEN** a template field reports an error
- **THEN** its message is visually distinguishable and programmatically associated with that field
- **AND** focus and editing remain available

### Requirement: Overlays keep keyboard ownership through dismissal

Opening the action panel SHALL preserve the underlying selected item and give the panel exclusive ownership of navigation and confirmation. Closing it SHALL restore focus to the invoking view's appropriate control. A visual exit SHALL NOT allow the closing key or click to invoke an underlying command or dismiss the launcher unintentionally.

#### Scenario: Open and close the action panel on Windows

- **WHEN** the user opens the panel with Ctrl+K, navigates with arrow keys, and presses Escape
- **THEN** only the panel closes, its parent selection is unchanged, and typing reaches the previous search field or form control

#### Scenario: Open and close the action panel on macOS

- **WHEN** the user opens the panel with Cmd+K or Ctrl+K, navigates with arrow keys, and presses Escape
- **THEN** only the panel closes, its parent selection is unchanged, and typing reaches the previous search field or form control

#### Scenario: Closing feedback cannot activate the parent

- **WHEN** the user confirms a selected panel action and its exit feedback begins
- **THEN** that action runs once and the same input does not also invoke the underlying selected result
- **AND** the outgoing panel does not accept another activation while closing

#### Scenario: Detail view regains keyboard ownership

- **WHEN** the action panel closes over a detail view that has no input
- **THEN** focus returns to that view and its existing shortcuts remain usable

### Requirement: Motion communicates state without delaying work

Launcher activation and keyboard selection SHALL NOT wait for or travel through an animation. Pointer hover and action-panel entrances and exits SHALL use restrained transitions. Result changes and full-tree replacements SHALL NOT replay entrance animations, stagger results, or animate selection travel.

The interface SHALL show no looping decorative effect when idle. Ongoing work SHALL use at most one activity sweep per visible busy surface, alongside its non-animated status information.

#### Scenario: Activation stays immediate on Windows

- **WHEN** the launcher is summoned for its first or a repeat activation on Windows
- **THEN** it is painted within 80 milliseconds of the global shortcut being received
- **AND** it does not start invisible or displaced by an entrance effect

#### Scenario: Activation stays immediate on macOS

- **WHEN** the launcher is summoned for its first or a repeat activation on macOS
- **THEN** it is painted within 80 milliseconds of the global shortcut being received
- **AND** it does not start invisible or displaced by an entrance effect

#### Scenario: Overlay feedback is short and non-blocking

- **WHEN** the action panel opens or closes with reduced motion disabled
- **THEN** each entrance or exit transition lasts no more than 180 milliseconds
- **AND** opening feedback does not delay keyboard navigation

#### Scenario: A streamed view stays stable

- **WHEN** a command emits thirty replacements of the same view over one second
- **THEN** content updates without replayed entrances, flicker, or animation-induced dropped frames
- **AND** selection and focus are not reset by the visual treatment

#### Scenario: Idle and hidden surfaces do not animate

- **WHEN** work finishes or the launcher is hidden
- **THEN** the activity sweep stops and no decorative loop remains running for that surface

### Requirement: Reduced motion retains all state information

Dango SHALL honor the operating system's reduced-motion preference, including changes while running. Reduced motion SHALL remove activity sweeps, shimmer, repeated pulses, and entrance or exit movement without removing loading, focus, selection, or error information.

#### Scenario: Reduced motion on Windows

- **WHEN** Windows exposes a reduced-motion preference while a busy view or action panel is visible
- **THEN** moving and looping effects stop, the panel remains usable, and a static busy indicator and status text remain available

#### Scenario: Reduced motion on macOS

- **WHEN** macOS exposes a reduced-motion preference while a busy view or action panel is visible
- **THEN** moving and looping effects stop, the panel remains usable, and a static busy indicator and status text remain available

### Requirement: Presentation fits the launcher viewport

Content and floating controls SHALL fit within the launcher viewport at supported window sizes and display scaling. Long content SHALL wrap or truncate appropriately without displacing primary controls or creating horizontal overflow. Overflowing action lists SHALL scroll while keeping the selected action visible.

#### Scenario: Compact viewport and long content

- **WHEN** the launcher renderer is previewed at 480 by 300 CSS pixels with long result titles, subtitles, actions, and form help
- **THEN** the footer and active controls remain reachable, floating content stays within the viewport, and no horizontal scrollbar appears

#### Scenario: Long action list

- **WHEN** the panel has more actions than fit vertically and the user navigates to the last action
- **THEN** the panel scrolls that action into view without moving selection in the parent list

#### Scenario: High display scaling

- **WHEN** the launcher is shown at 200 percent display scaling on either supported platform
- **THEN** panel corners, icons, text, and focus indicators remain legible and are not clipped by the native window edge
