# Spec Delta

## Purpose

Answer the root search query itself when it is math or a unit conversion, in a
card above the results, so the answer is one key away.

## ADDED Requirements

### Requirement: A query that is math gets an answer

Root search SHALL show an answer when the query contains an operator (+, -, *,
/, ^, %) or the words "to", "in", or "as" between values, and Dango can work
it out. A dot SHALL be the decimal separator in the query and the answer. A
query with a comma between digits, or without an answer, SHALL show no answer
card. Working out an answer SHALL NOT delay typing or the results.

#### Scenario: Arithmetic

- **WHEN** the user types "2+2" at root
- **THEN** an answer card shows "4"

#### Scenario: A unit conversion

- **WHEN** the user types "5 km to m" at root
- **THEN** an answer card shows "5000 m"

#### Scenario: A decimal

- **WHEN** the user types "1.5 * 2" at root
- **THEN** an answer card shows "3"

#### Scenario: A comma as the decimal separator

- **WHEN** the user types "1,5*2" at root
- **THEN** no answer card is shown

#### Scenario: Names that hold a digit

- **WHEN** the user types "firefox", "pi", "4k", or "2fa" at root
- **THEN** no answer card is shown

### Requirement: The answer card sits above the results

The answer card SHALL show between the SearchBar and the results, with the
query's expression on the left and the answer on the right. It is the first row
of root search (see list-navigation). When no result matches, the card SHALL
show in place of the empty view. Turning the calculator off SHALL remove the
card.

#### Scenario: The card is selected

- **WHEN** the user types a query that gets an answer
- **THEN** the card is selected and the first result is not

#### Scenario: No result matches

- **WHEN** the user types "17*23" and no result matches
- **THEN** the answer card shows "391" and no "No results" view is shown

### Requirement: An approximate answer says so on the card only

When an answer is approximate, the card SHALL show it after "≈". Copying and
pasting SHALL give the answer without that mark.

#### Scenario: An approximate conversion

- **WHEN** the user types "5 km in miles" at root and presses Enter
- **THEN** the card shows "≈ 3.1068559612 miles" and the clipboard holds
  "3.1068559612 miles"

### Requirement: Enter copies the answer

With the card selected, the ActionBar SHALL show "Calculator" as its context and
"Copy answer" with the Enter key. Enter SHALL copy the answer and close the
launcher with the copied HUD. The action panel SHALL also offer "Paste answer",
which pastes the answer into the application the user came from. Copying or
pasting an answer SHALL NOT change how root results rank.

#### Scenario: Copying

- **WHEN** the card shows "4" and the user presses Enter
- **THEN** "4" is on the clipboard and the HUD reads "Copied to clipboard"

#### Scenario: Pasting

- **WHEN** the card shows "4" and the user runs "Paste answer" from the panel
- **THEN** the launcher hides and "4" is pasted into the application that had
  focus before

#### Scenario: An answer is not learned

- **WHEN** the user pastes the answer to "2+2" and later types "2+"
- **THEN** no result ranks higher because of that paste

