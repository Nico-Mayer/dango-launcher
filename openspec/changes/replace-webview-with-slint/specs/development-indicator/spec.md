## MODIFIED Requirements

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

### Requirement: Both states are testable without a Tauri process

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
