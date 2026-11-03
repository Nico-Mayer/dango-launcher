# Spec Delta

## Purpose

Answer the root search query itself when it is math or a unit conversion, in a
card above the results, so the answer is one key away.

## ADDED Requirements

### Requirement: A query that is math gets an answer

Root search SHALL show an answer when the query is arithmetic or a unit
conversion that Dango can work out, contains a digit or an operator, and has an
answer that differs from the query. A dot SHALL be the decimal separator in the
query and in the answer on every machine. A query without an answer SHALL show
no answer card. Working out an answer SHALL NOT delay typing or the results.

#### Scenario: Arithmetic

- **WHEN** the user types "2+2" at root
- **THEN** an answer card shows "4"

#### Scenario: A unit conversion

- **WHEN** the user types "5 km to m" at root
- **THEN** an answer card shows "5000 m"

#### Scenario: A decimal

- **WHEN** the user types "1.5 * 2" at root
- **THEN** an answer card shows "3"

#### Scenario: An application name

- **WHEN** the user types "firefox" at root
- **THEN** no answer card is shown

#### Scenario: A lone constant

- **WHEN** the user types "pi" at root
- **THEN** no answer card is shown, because the query holds no digit or operator

### Requirement: The answer card sits above the results

The answer card SHALL show between the SearchBar and the results, with the
query's expression on the left and the answer on the right. It SHALL count as
the row above the first result. A new answer SHALL select the card. When no
result matches, the card SHALL show in place of the empty view.

#### Scenario: The card is selected

- **WHEN** an answer arrives for the query
- **THEN** the card is selected and the first result is not

#### Scenario: Moving into the results

- **WHEN** the card is selected and the user presses Arrow Down
- **THEN** the first result is selected

#### Scenario: No result matches

- **WHEN** the user types "17*23" and no result matches
- **THEN** the answer card shows "391" and no "No results" view is shown

### Requirement: Enter copies the answer

With the card selected, the ActionBar SHALL show "Calculator" as its context and
"Copy answer" with the Enter key. Enter SHALL copy the answer to the clipboard
and close the launcher with the copied HUD. The action panel SHALL also offer
"Paste answer", which pastes the answer into the application the user came
from.

#### Scenario: Copying

- **WHEN** the card shows "4" and the user presses Enter
- **THEN** "4" is on the clipboard and the HUD reads "Copied to clipboard"

#### Scenario: Pasting

- **WHEN** the card shows "4" and the user runs "Paste answer" from the panel
- **THEN** the launcher hides and "4" is pasted into the application that had
  focus before
