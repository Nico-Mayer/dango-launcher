# development-indicator Specification

## Purpose

Lets the author tell a development build of Dango from the installed release
at a glance, in the launcher window and in the tray, so a change under test is
judged against the right build.

## Requirements

### Requirement: A development build is marked in the launcher window

When Dango runs as a development build, the launcher window SHALL show a
visible development marker: the frame edge SHALL use a distinct development
colour, and the footer SHALL show a short "dev" badge next to the product
name. A release build SHALL show neither. "Development build" means a debug
build, the same meaning the autostart and latency behaviour already use.

The marker SHALL be drawn inside the launcher window's own content, SHALL NOT
change the window's system chrome, size, or layout density, and SHALL be
present on every launcher surface: root search, pushed views, and the protocol
error screen.

#### Scenario: Development build on Windows

- **WHEN** a debug build of Dango is summoned on Windows
- **THEN** the frame edge is drawn in the development colour and the footer
  shows the "dev" badge
- **AND** result rows, the search header, and the footer keep their default
  heights

#### Scenario: Development build on macOS

- **WHEN** a debug build of Dango is summoned on macOS
- **THEN** the frame edge is drawn in the development colour and the footer
  shows the "dev" badge
- **AND** result rows, the search header, and the footer keep their default
  heights

#### Scenario: Release build shows nothing

- **WHEN** a release build of Dango is summoned on either platform
- **THEN** the frame edge uses the default edge colour and no "dev" badge is
  shown

#### Scenario: Marker survives a pushed view

- **WHEN** a development build shows a pushed list, detail, or form view, or
  the protocol error screen
- **THEN** the frame edge still uses the development colour

#### Scenario: Marker follows the shared presentation

- **WHEN** the development colour is changed in the built-in palette
- **THEN** the frame edge uses the new value
- **AND** the badge remains readable against the footer in the dark palette

### Requirement: The marker does not cost activation time

The renderer SHALL learn whether it runs in a development build from the
launcher state it already receives, and SHALL NOT make a separate request for
it, on activation or otherwise. The answer SHALL come from the application,
not from how the frontend was served.

#### Scenario: Asked once at mount

- **WHEN** startup completes and the launcher has not been shown yet
- **THEN** the renderer already knows whether this is a development build
- **AND** showing and hiding the launcher afterwards sends no request for it

#### Scenario: Activation latency is unchanged

- **WHEN** a development build is summoned after warm-up
- **THEN** the launcher is painted without added delay, with the marker
  already in place

### Requirement: A development build is named in the tray

The tray icon of a development build SHALL carry a tooltip that names it as a
development build. A release build's tray icon SHALL carry a tooltip with the
product name only.

#### Scenario: Hovering the tray icon on Windows

- **WHEN** the pointer rests on the tray icon of a debug build on Windows
- **THEN** the tooltip reads "Dango (development build)"

#### Scenario: Hovering the tray icon on macOS

- **WHEN** the pointer rests on the menu bar icon of a debug build on macOS
- **THEN** the tooltip reads "Dango (development build)"

#### Scenario: Release tray tooltip

- **WHEN** the pointer rests on the tray icon of a release build
- **THEN** the tooltip reads "Dango"

### Requirement: Both states are testable without a window

The automated UI tests SHALL be able to mount the launcher in either the
development or the release state without a visible window, so each
appearance is checked by `cargo test`.

#### Scenario: Fixture mounts the development state

- **WHEN** a headless UI test mounts the launcher in the development state
- **THEN** the frame carries the development marker and the footer shows the
  badge

#### Scenario: Fixture default is the release state

- **WHEN** a headless UI test mounts the launcher without the development
  state
- **THEN** no marker and no badge are shown
