# window-management Specification

## Purpose

Reshaping the window the user was in before the launcher appeared: tiling it to a
half, quarter, or third of its display, maximising or centring it, and moving it
to the next display, so common window arrangement happens from the keyboard
without touching the mouse.

## Requirements

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
its width or height. Repeating the command on a window it has just centred
enters the size cycle instead, as the cycling requirement below describes.

#### Scenario: Centring a window

- **WHEN** the user invokes the centre command on a window it has not just
  centred
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

### Requirement: Repeating a tiling command cycles its size

When the user invokes a cycling command and the target window is still where that
same command last placed it, Dango SHALL advance the window to the next size in
the cycle (1/2, then 2/3, then 1/3, wrapping back to 1/2) rather than leaving it
unchanged. Otherwise Dango SHALL place the window at the command's base result,
which is its existing single-press behaviour: the half commands at 1/2, and
centre at the window's current size. Left and right half cycle their width; top
and bottom half cycle their height; centre cycles both dimensions, staying
centred.

Because centre's base result is the window's current size, existing single-press
behaviour is unchanged for every command.

#### Scenario: Repeating left half narrows it through the cycle

- **WHEN** the user invokes left half three times in a row on a window that stays
  put between presses
- **THEN** the window occupies the left 1/2, then the left 2/3, then the left 1/3
  of the work area
- **AND** a fourth invocation returns it to the left 1/2

#### Scenario: A single press is unchanged

- **WHEN** the user invokes left half once
- **THEN** the window occupies the left half of the work area, as before

#### Scenario: Moving the window resets the cycle

- **WHEN** the user invokes left half, then moves or resizes the window by any
  other means, then invokes left half again
- **THEN** the window occupies the left 1/2, not the next size in the cycle

#### Scenario: Switching commands starts a new cycle

- **WHEN** the user invokes left half and then invokes right half
- **THEN** right half places the window at the right 1/2, starting its own cycle

#### Scenario: Centre keeps size on a fresh press, then cycles

- **WHEN** the user invokes centre on a window it has not just centred
- **THEN** the window is centred at its current size, unchanged from before
- **AND** invoking centre again on the unmoved window centres it at 1/2, then
  2/3, then 1/3 of the work area in both dimensions

### Requirement: The focused window can be sized to a comfortable default

Dango SHALL offer commands that size the focused window to a roomy or a
comfortable fraction of the work area, centred, regardless of its current size.

#### Scenario: Almost maximise

- **WHEN** the user invokes almost maximise
- **THEN** the window is centred and fills most of the work area with a small
  margin on every side
- **AND** it does not cover the taskbar, dock, or menu bar

#### Scenario: Reasonable size

- **WHEN** the user invokes reasonable size
- **THEN** the window is centred at a comfortable fraction of the work area,
  smaller than almost maximise

### Requirement: The focused window can fill a centred column

Dango SHALL offer a command that centres the focused window as a column: the
middle portion of the work area's width, at its full height.

#### Scenario: Centre half

- **WHEN** the user invokes centre half
- **THEN** the window occupies the middle half of the work area's width and its
  full height

### Requirement: The focused window can be grown and shrunk by steps

Dango SHALL offer commands that make the focused window larger or smaller by a
step, expanding or contracting it around its own centre.

#### Scenario: Make larger

- **WHEN** the user invokes make larger
- **THEN** the window grows by a step on every side, staying centred on where it
  was
- **AND** it never grows beyond the work area

#### Scenario: Make smaller

- **WHEN** the user invokes make smaller
- **THEN** the window shrinks by a step on every side, staying centred on where
  it was
- **AND** it never shrinks below a usable minimum size

#### Scenario: Repeated steps accumulate

- **WHEN** the user invokes make larger several times
- **THEN** the window grows a step each time until it reaches the work area, then
  stops growing
