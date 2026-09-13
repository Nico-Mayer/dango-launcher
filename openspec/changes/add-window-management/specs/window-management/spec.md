## Purpose

Reshaping the window the user was in before the launcher appeared: tiling it to a
half, quarter, or third of its display, maximising or centring it, and moving it
to the next display, so common window arrangement happens from the keyboard
without touching the mouse.

## ADDED Requirements

### Requirement: The focused window can be tiled to a region of its display

Dango SHALL move and resize the target window so it exactly fills a named region
of its display's work area. The regions are the left, right, top, and bottom
halves; the four quarters; the left, centre, and right thirds; and the whole
work area (maximise).

The target window is the window that was focused before the launcher appeared,
never the launcher's own window.

#### Scenario: Tiling to the left half

- **WHEN** the user invokes the left-half command with a window focused
- **THEN** that window fills the left half of its display's work area
- **AND** its top and bottom edges meet the top and bottom of the work area

#### Scenario: Tiling to a quarter

- **WHEN** the user invokes the top-left-quarter command
- **THEN** the window fills the top-left quarter of the work area

#### Scenario: Tiling to a third

- **WHEN** the user invokes the centre-third command
- **THEN** the window fills the middle third of the work area horizontally and
  the full height of the work area

#### Scenario: Maximising

- **WHEN** the user invokes the maximise command
- **THEN** the window fills the whole work area
- **AND** it does not cover the taskbar, dock, or menu bar

#### Scenario: The launcher's own window is never the target

- **WHEN** a window-management command runs while the launcher is on screen
- **THEN** the window that was focused before the launcher appeared is the one
  that moves
- **AND** the launcher window is not moved or resized

### Requirement: Centring keeps the window's size

Dango SHALL centre the target window on its display's work area without changing
its width or height.

#### Scenario: Centring a window

- **WHEN** the user invokes the centre command
- **THEN** the window is positioned at the centre of the work area
- **AND** its width and height are unchanged

### Requirement: Windows are placed flush against the work area

A tiled window SHALL sit flush against the edges of the region with no visible
gap, and SHALL respect the display's reserved areas rather than the full screen
bounds.

#### Scenario: No gap on Windows

- **WHEN** a window is tiled to the left half on Windows
- **THEN** its visible edges are flush with the work area with no gap
- **AND** the placement accounts for the invisible resize border Windows reports
  around the frame

#### Scenario: Reserved areas are respected

- **WHEN** a window is maximised on either platform
- **THEN** it fills the work area only, leaving the taskbar, dock, or menu bar
  visible

### Requirement: A window can be moved to the next display

When more than one display is present, Dango SHALL move the target window to the
next display and place it in the region there that matches where it was.

#### Scenario: Moving to the next display

- **WHEN** the user invokes move-to-next-display with two or more displays present
- **THEN** the window moves to the next display in order
- **AND** it occupies the same relative region of that display's work area it
  occupied before

#### Scenario: A single display is a no-op

- **WHEN** the user invokes move-to-next-display with only one display present
- **THEN** the window stays where it is
- **AND** nothing reports an error

#### Scenario: Crossing displays of different scale

- **WHEN** the window is moved to a display with a different DPI scale
- **THEN** it fills the intended region of the destination display's work area
  correctly, not scaled by the display it left

### Requirement: A window arrangement is fast enough to feel instant

A window-management command SHALL move and resize the target window quickly
enough that it feels immediate rather than animated or laggy.

#### Scenario: The window moves promptly

- **WHEN** the user confirms a window-management command
- **THEN** the window has reached its new position and size within 100ms

### Requirement: Platform permission and gating

On Windows the commands need no permission. On macOS both moving and resizing a
window require the Accessibility permission, and Dango SHALL detect its absence
and explain it rather than doing nothing.

#### Scenario: Acting on Windows

- **WHEN** the user invokes a window-management command on Windows
- **THEN** the window moves with no permission step

#### Scenario: Acting without the permission on macOS

- **WHEN** the user invokes a window-management command on macOS and the
  Accessibility permission has not been granted
- **THEN** no window is moved
- **AND** the launcher explains that the permission is needed and offers to
  request it

#### Scenario: The permission is granted on macOS

- **WHEN** the Accessibility permission has been granted
- **THEN** window-management commands work without any further prompting

### Requirement: Failures are reported, never silent

A command that cannot reshape the target SHALL leave a message the user can act
on rather than failing silently, and SHALL leave the window as it found it.

#### Scenario: There is no window to move

- **WHEN** a window-management command runs and there is no previous window to
  act on
- **THEN** the command fails with a message saying so
- **AND** nothing on screen is moved

#### Scenario: The target window is elevated on Windows

- **WHEN** the target window belongs to an elevated process and Dango is not
  elevated
- **THEN** the command fails with a message saying the window cannot be reached
- **AND** the window is left unchanged
