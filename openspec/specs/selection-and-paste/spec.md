# selection-and-paste Specification

## Purpose

The one path between Dango and whatever application the user was in: reading the
text they have selected, and delivering text back so it arrives as though they
had pasted it themselves, without them ever noticing that their clipboard was
borrowed to do it.

## Requirements

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
- **AND** the clipboard holds what it held before the read, once the read has
  finished

#### Scenario: Reading a selection is fast enough to act on

- **WHEN** a selection of 1,000 characters is read
- **THEN** it is returned within 300ms

#### Scenario: An application that does not answer in time

- **WHEN** the frontmost application has not answered the direct request within
  200ms
- **THEN** Dango stops waiting for it
- **AND** the read completes by the fallback or reports that it could not be done,
  within the 300ms budget

### Requirement: Text is delivered into the frontmost application

Dango SHALL be able to insert text into the application the user was in before
the launcher appeared, arriving in the focused text field as though the user had
pasted it.

#### Scenario: Text arrives in the application on macOS

- **WHEN** the user invokes something that inserts text while an editable field is focused in another application
- **THEN** the launcher is no longer on screen before the text is sent
- **AND** the text appears in that field

#### Scenario: Text arrives in the application on Windows

- **WHEN** the user invokes something that inserts text while an editable field is focused in another application
- **THEN** the launcher is no longer on screen and the previous window is the foreground window before the text is sent
- **AND** the text appears in that field

#### Scenario: The previous window is never skipped over on Windows

- **WHEN** the previous window has not become the foreground window yet
- **THEN** no text is sent until it has
- **AND** if it never does, nothing is sent and the failure is reported

#### Scenario: Text appears promptly

- **WHEN** the user confirms an action that inserts text
- **THEN** the text has appeared in the target application within 400ms

#### Scenario: The caret can be placed inside the inserted text

- **WHEN** text is inserted that declares where the caret should end up
- **THEN** the whole text is inserted
- **AND** the caret is left at the declared position rather than at the end

### Requirement: The user's clipboard survives

Reading a selection and inserting text MAY use the clipboard, and the user SHALL
NOT be able to observe that it did. The clipboard's contents SHALL be the same
before and after, for text and for images alike.

The one exception is pasting an entry from the clipboard history. There the
user has chosen content that lives on the clipboard, so the entry SHALL be left
on the clipboard after it is pasted, ready to be pasted again.

#### Scenario: The clipboard is restored after an insertion

- **WHEN** the user has content on the clipboard and Dango inserts text
- **THEN** the clipboard holds that same content once the insertion has finished
- **AND** it is restored within 1 second

#### Scenario: An image on the clipboard survives

- **WHEN** the clipboard holds an image and Dango inserts text
- **THEN** the clipboard holds that same image afterwards

#### Scenario: The user copies something during the operation

- **WHEN** the clipboard's contents change between Dango saving them and restoring them
- **THEN** the restore is abandoned
- **AND** what the user copied is left on the clipboard

#### Scenario: A pasted history entry stays on the clipboard

- **WHEN** the user pastes an entry from the clipboard history
- **THEN** the clipboard holds that entry once the paste has finished
- **AND** pasting again in the application pastes the same entry

### Requirement: Dango's own clipboard use never reaches the history

No clipboard write Dango makes in order to read a selection or insert text SHALL
appear in the clipboard history, and no such write SHALL reorder an entry that
is already there.

#### Scenario: Inserted text is not recorded

- **WHEN** Dango inserts text while the clipboard history is recording
- **THEN** that text does not appear in the history

#### Scenario: Restoring the clipboard does not reorder the history

- **WHEN** Dango restores the user's clipboard after inserting text
- **THEN** the history is in the same order it was in before

#### Scenario: Reading a selection through the clipboard is not recorded

- **WHEN** Dango reads a selection by copying it
- **THEN** the copied text does not appear in the history

#### Scenario: A genuine copy during the operation is still recorded

- **WHEN** the user copies something themselves while Dango is inserting text
- **THEN** what they copied is recorded in the history

### Requirement: The macOS permission is a visible state

On macOS both reading the selection and inserting text require the Accessibility
permission. Dango SHALL detect that it is missing and SHALL explain it, rather
than failing silently.

#### Scenario: Acting without the permission on macOS

- **WHEN** the user invokes something that reads the selection or inserts text and the permission has not been granted
- **THEN** nothing is sent to the other application
- **AND** the launcher explains that the permission is needed and offers to request it

#### Scenario: Requesting the permission on macOS

- **WHEN** the user chooses to request the permission
- **THEN** the system's permission prompt is shown
- **AND** no prompt appears at any other time

#### Scenario: The permission is granted on macOS

- **WHEN** the permission has been granted
- **THEN** reading the selection and inserting text work without any further prompting

#### Scenario: Windows needs no permission

- **WHEN** the user invokes something that reads the selection or inserts text on Windows
- **THEN** it works with no permission step

### Requirement: Failures are reported, never silent

An insertion or a read that cannot be completed SHALL leave a message the user
can act on, and SHALL leave the clipboard as it found it.

#### Scenario: The target window is elevated on Windows

- **WHEN** the frontmost window belongs to an elevated process
- **THEN** the insertion fails with a message saying so
- **AND** the clipboard is unchanged

#### Scenario: There is no application to insert into

- **WHEN** there is no previous application to return to
- **THEN** the insertion fails with a message saying so
- **AND** the clipboard is unchanged

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
