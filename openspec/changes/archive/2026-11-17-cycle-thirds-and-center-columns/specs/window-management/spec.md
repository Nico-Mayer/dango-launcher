# Spec Delta

## MODIFIED Requirements

### Requirement: A repeated tiling command cycles its size

When the user invokes a cycling command and the target window is still where that
same command last placed it, Dango SHALL advance the window to the next size in
the ring 1/2, 2/3, 1/3 (1/3 is followed by 1/2) rather than leaving it
unchanged. Otherwise Dango SHALL place the window at the command's base result:
the half commands and centre half at 1/2, the third commands at 1/3, and centre
at the window's current size. Left and right half cycle their width; top and
bottom half cycle their height; centre cycles both dimensions, staying centred.

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

## ADDED Requirements

### Requirement: Third and centred-column commands cycle their width

Left third, right third, centre third, and centre half SHALL be cycling
commands that change only the window's width, at full height. Left and right
third stay against their edge; centre third and centre half stay centred
horizontally. A size reached by repeating one command SHALL be the same
rectangle as that size placed by the command named for it.

#### Scenario: Repeating left third widens it through the ring

- **WHEN** the user invokes left third three times in a row on a window that
  stays put between presses
- **THEN** the window occupies the left 1/3, then the left 1/2, then the left
  2/3 of the work area, at full height
- **AND** a fourth invocation returns it to the left 1/3

#### Scenario: Right third stays against the right edge

- **WHEN** the user invokes right third twice in a row on a window that stays
  put between presses
- **THEN** the window occupies the right 1/3, then the right 1/2 of the work
  area
- **AND** its right edge meets the right edge of the work area both times

#### Scenario: Repeating centre third widens the centred column

- **WHEN** the user invokes centre third three times in a row on a window that
  stays put between presses
- **THEN** the window occupies the middle 1/3, then the middle 1/2, then the
  middle 2/3 of the work area's width, at full height
- **AND** it is centred horizontally each time, within a pixel

#### Scenario: Repeating centre half cycles the centred column

- **WHEN** the user invokes centre half three times in a row on a window that
  stays put between presses
- **THEN** the window occupies the middle 1/2, then the middle 2/3, then the
  middle 1/3 of the work area's width, at full height
- **AND** a fourth invocation returns it to the middle 1/2

#### Scenario: A first press of each third still tiles the work area

- **WHEN** three windows are placed with one press each of left third, centre
  third, and right third
- **THEN** together they cover the work area's width with no gap and no overlap

#### Scenario: A size reached by repeating matches the command named for it

- **WHEN** one window is placed with two presses of right third and another
  with one press of right half
- **THEN** both occupy exactly the same rectangle
- **AND** the same holds for three presses of centre half and one press of
  centre third

#### Scenario: Cycled sizes tile with the opposite side

- **WHEN** one window is placed with three presses of left third and another
  with one press of right third
- **THEN** the left 2/3 and the right 1/3 meet with no gap and no overlap
