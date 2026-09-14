## MODIFIED Requirements

### Requirement: The selected text can be read

Dango SHALL be able to read the text currently selected in the frontmost
application, and SHALL report the absence of a selection distinctly from a
failure to read one.

On both platforms Dango SHALL first ask the application directly, through the
platform's accessibility interface, and SHALL send no keystrokes to do it. The
clipboard round trip SHALL be a fallback, used only where the application cannot
be asked.

An application that answers but reports nothing selected SHALL be treated as an
empty selection, not as a failure to read, and SHALL NOT fall through to the
keystroke path.

#### Scenario: Reading a selection on macOS

- **WHEN** text is selected in the frontmost application and Dango reads the selection
- **THEN** that text is returned
- **AND** the clipboard's contents are unchanged

#### Scenario: Reading a selection on Windows

- **WHEN** text is selected in the frontmost application, that application
  exposes it, and Dango reads the selection
- **THEN** that text is returned
- **AND** no keystroke is sent to the application
- **AND** the clipboard's contents are unchanged

#### Scenario: Reading a selection on Windows from an application that exposes nothing

- **WHEN** the frontmost application cannot be asked what is selected
- **THEN** Dango falls back to reading it through the clipboard
- **AND** the clipboard holds what it held before the read, once the read has finished

#### Scenario: Nothing is selected

- **WHEN** Dango reads the selection and the frontmost application has none
- **THEN** an empty selection is reported
- **AND** anything acting on the selection says so rather than acting on empty text

#### Scenario: The application answers and has no selection

- **WHEN** the frontmost application exposes its text but nothing is selected
- **THEN** an empty selection is reported
- **AND** no keystroke is sent

#### Scenario: The application will not say what is selected

- **WHEN** the frontmost application cannot be asked what is selected
- **THEN** Dango falls back to reading it through the clipboard
- **AND** the clipboard is restored afterwards

#### Scenario: Reading a selection is fast enough to act on

- **WHEN** a selection of 1,000 characters is read
- **THEN** it is returned within 300ms

#### Scenario: An application that does not answer in time

- **WHEN** the frontmost application has not answered the direct request within
  200ms
- **THEN** Dango stops waiting for it
- **AND** the read completes by the fallback or reports that it could not be done,
  within the 300ms budget

## ADDED Requirements

### Requirement: The keystroke fallback can be refused for an application

A synthesized copy is a chord the target application is free to bind to
something else, and a modal editor does: in a Helix keymap Ctrl+C toggles
comments, so the fallback edits the user's document instead of copying it.

The user SHALL be able to name applications where the clipboard fallback must
not be attempted. For a named application Dango SHALL report that it cannot read
the selection, and SHALL send no keystroke. Applications SHALL be named the way
the clipboard history already names its exclusions, so one habit covers both.

#### Scenario: A named application is never sent the copy chord

- **WHEN** the frontmost application is one the user named, and something reads
  the selection
- **THEN** no keystroke is sent to it
- **AND** the failure says the selection could not be read there

#### Scenario: A named application that does expose its selection still works

- **WHEN** a named application exposes its selection through the accessibility
  interface
- **THEN** the selection is read directly
- **AND** the exclusion never comes into play

#### Scenario: An application that is not named is unaffected

- **WHEN** the frontmost application is not named and does not expose its
  selection
- **THEN** the clipboard fallback runs as before

#### Scenario: The list is empty by default

- **WHEN** the user has named no applications
- **THEN** every application falls back as it does today
