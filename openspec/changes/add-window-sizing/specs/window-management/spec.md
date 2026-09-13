## ADDED Requirements

### Requirement: Repeating a tiling command cycles its size

When the user invokes a cycling command and the target window is still where that
same command last placed it, Dango SHALL advance the window to the next size in
the cycle rather than leaving it unchanged. The cycle is 1/2, then 2/3, then 1/3,
then back to 1/2. Left and right half cycle their width; top and bottom half
cycle their height; centre cycles both dimensions, staying centred.

The first invocation, or any invocation after the window has changed since the
command last acted on it, SHALL place the window at the first size in the cycle
(1/2), so existing behaviour is unchanged for a single press.

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

#### Scenario: Centre cycles both dimensions

- **WHEN** the user invokes centre repeatedly on a window that stays put
- **THEN** the window is centred at 1/2, then 2/3, then 1/3 of the work area in
  both width and height

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
