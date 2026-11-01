## MODIFIED Requirements

### Requirement: Windows are placed flush against the work area

A tiled window SHALL sit flush against the edges of the region with no visible
gap, and SHALL respect the display's reserved areas rather than the full screen
bounds.

#### Scenario: No gap on Windows

- **WHEN** a window is tiled to the left half on Windows
- **THEN** its visible edges are flush with the work area with no gap

#### Scenario: No gap on macOS

- **WHEN** a window is tiled to the left half on macOS
- **THEN** its visible edges are flush with the work area with no gap

#### Scenario: Reserved areas are respected

- **WHEN** a window is maximised on either platform
- **THEN** it fills the work area only, leaving the taskbar, dock, or menu bar
  visible

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
- **AND** Dango asks macOS to show its Accessibility prompt
- **AND** the command fails with "Allow Accessibility in System Settings"

#### Scenario: The permission is granted on macOS

- **WHEN** the Accessibility permission has been granted
- **THEN** window-management commands work without any further prompting

## REMOVED Requirements

### Requirement: Repeating a tiling command cycles its size

**Reason**: Replaced by "A repeated tiling command cycles its size". Change-era wording ("as before", "existing single-press behaviour") is rewritten as current behaviour. OpenSpec cannot rename or drop a
scenario inside a MODIFIED block.

**Migration**: None. The rule is unchanged under the new header.

## ADDED Requirements

### Requirement: A repeated tiling command cycles its size

When the user invokes a cycling command and the target window is still where that
same command last placed it, Dango SHALL advance the window to the next size in
the cycle (1/2, then 2/3, then 1/3, wrapping back to 1/2) rather than leaving it
unchanged. Otherwise Dango SHALL place the window at the command's base result:
the half commands at 1/2, and centre at the window's current size. Left and
right half cycle their width; top and bottom half cycle their height; centre
cycles both dimensions, staying centred.

#### Scenario: Repeating left half narrows it through the cycle

- **WHEN** the user invokes left half three times in a row on a window that stays
  put between presses
- **THEN** the window occupies the left 1/2, then the left 2/3, then the left 1/3
  of the work area
- **AND** a fourth invocation returns it to the left 1/2

#### Scenario: A single press places the half

- **WHEN** the user invokes left half once
- **THEN** the window occupies the left half of the work area

#### Scenario: Moving the window resets the cycle

- **WHEN** the user invokes left half, then moves or resizes the window by any
  other means, then invokes left half again
- **THEN** the window occupies the left 1/2, not the next size in the cycle

#### Scenario: Switching commands starts a new cycle

- **WHEN** the user invokes left half and then invokes right half
- **THEN** right half places the window at the right 1/2, starting its own cycle

#### Scenario: Centre keeps size on a fresh press, then cycles

- **WHEN** the user invokes centre on a window it has not just centred
- **THEN** the window is centred at its current size
- **AND** invoking centre again on the unmoved window centres it at 1/2, then
  2/3, then 1/3 of the work area in both dimensions
