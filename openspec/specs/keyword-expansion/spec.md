# keyword-expansion Specification

## Purpose

Typing a snippet's keyword in any application and having the snippet's text
appear in its place, with nothing on screen. Also, and mostly, what the thing
watching those keystrokes is allowed to see, allowed to keep, and required to
forget.

## Requirements

### Requirement: A typed keyword is replaced by its snippet

When the user types a snippet's keyword in any application, Dango SHALL remove
the typed keyword and insert that snippet's text in its place.

#### Scenario: A keyword expands

- **WHEN** the user types a snippet's keyword into an editable field
- **THEN** the keyword is removed
- **AND** the snippet's text appears where the keyword was

#### Scenario: The caret ends after the inserted text

- **WHEN** a snippet without a declared caret position is expanded
- **THEN** the caret is left at the end of the inserted text

#### Scenario: A snippet declaring a caret position

- **WHEN** an expanded snippet declares where the caret should end up
- **THEN** the caret is left at that position

#### Scenario: Expansion is fast enough to keep typing through

- **WHEN** a keyword is completed
- **THEN** the replacement has happened within 500ms

#### Scenario: The user's clipboard is unaffected

- **WHEN** a keyword expands
- **THEN** the clipboard holds what it held before
- **AND** nothing about the expansion appears in the clipboard history

### Requirement: A keyword only matches as a whole word

A keyword SHALL match only when the character before it is not a word
character, so that a keyword occurring inside a longer word does not expand.

#### Scenario: A keyword at the start of a line

- **WHEN** the user types the keyword with nothing before it
- **THEN** it expands

#### Scenario: A keyword after a space

- **WHEN** the user types the keyword directly after a space or punctuation
- **THEN** it expands

#### Scenario: A keyword inside a longer word

- **WHEN** the keyword's characters appear as part of a longer word the user is typing
- **THEN** nothing expands

#### Scenario: Only the exact keyword matches

- **WHEN** the user types something that differs from the keyword in any character, including its case
- **THEN** nothing expands

### Requirement: What is observed is bounded and forgotten

The component observing keystrokes SHALL retain no more characters than the
longest keyword requires, SHALL keep them in memory only, and SHALL never write
them anywhere.

#### Scenario: Nothing typed is ever stored

- **WHEN** the user types anything at all
- **THEN** no record of it exists in the database, in a file, or in a log

#### Scenario: What is held is bounded

- **WHEN** the user types a long passage
- **THEN** no more than the longest keyword's worth of recent characters is held at any moment

#### Scenario: A key that is not a character clears what is held

- **WHEN** the user presses Enter, Tab, Escape, an arrow key, or a chord with a modifier other than Shift
- **THEN** what was held is discarded

#### Scenario: Switching application clears what is held

- **WHEN** the frontmost application changes
- **THEN** what was held is discarded

#### Scenario: A pause clears what is held

- **WHEN** the user stops typing for several seconds
- **THEN** what was held is discarded

### Requirement: Expansion is skipped where it should not happen

Dango SHALL NOT observe or expand in contexts the user has excluded or the
system marks as sensitive.

#### Scenario: A password field on macOS

- **WHEN** the system reports that secure input is active
- **THEN** nothing is observed, nothing is held, and nothing expands

#### Scenario: A password field on Windows

- **WHEN** the user types into a password field
- **THEN** nothing expands
- **AND** the limitation that the system offers no reliable way to know this is documented for the user

#### Scenario: An excluded application

- **WHEN** the user types in an application on the exclusion list
- **THEN** nothing is observed and nothing expands

#### Scenario: The exclusion list is honoured without a restart

- **WHEN** the user changes the exclusion list
- **THEN** the change takes effect on the next keystroke observed

### Requirement: Observing never delays the machine

Observation SHALL NOT make typing perceptibly slower in any application, and
SHALL NOT interfere with the keystrokes it observes.

#### Scenario: Typing stays responsive

- **WHEN** the monitor is running and the user types continuously
- **THEN** no keystroke is delayed by more than 5ms by Dango's observation

#### Scenario: Keystrokes are never swallowed or altered

- **WHEN** any key is pressed, whether or not it completes a keyword
- **THEN** the application receives exactly the keystroke the user made

#### Scenario: The monitor recovers if the system stops it

- **WHEN** the operating system disables Dango's observation
- **THEN** Dango restores it
- **AND** expansion works again without a restart

### Requirement: Observation follows the extension's enabled state

Observation SHALL exist only while the snippets extension is enabled, and
disabling it SHALL remove the observation rather than leave it running.

#### Scenario: Disabling stops observation

- **WHEN** the user disables the snippets extension
- **THEN** nothing is observed at all
- **AND** no keyword expands

#### Scenario: Enabling starts observation

- **WHEN** the user enables the snippets extension
- **THEN** keywords expand again without a restart

#### Scenario: The permission is missing on macOS

- **WHEN** the Accessibility permission has not been granted
- **THEN** expansion does not work
- **AND** the user is told why rather than left with a feature that silently does nothing
