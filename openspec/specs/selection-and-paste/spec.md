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

#### Scenario: Reading a selection on macOS

- **WHEN** text is selected in the frontmost application and Dango reads the selection
- **THEN** that text is returned
- **AND** the clipboard's contents are unchanged

#### Scenario: Reading a selection on Windows

- **WHEN** text is selected in the frontmost application and Dango reads the selection
- **THEN** that text is returned
- **AND** the clipboard holds what it held before the read, once the read has finished

#### Scenario: Nothing is selected

- **WHEN** Dango reads the selection and the frontmost application has none
- **THEN** an empty selection is reported
- **AND** anything acting on the selection says so rather than acting on empty text

#### Scenario: The application will not say what is selected

- **WHEN** the frontmost application cannot be asked what is selected
- **THEN** Dango falls back to reading it through the clipboard
- **AND** the clipboard is restored afterwards

#### Scenario: Reading a selection is fast enough to act on

- **WHEN** a selection of 1,000 characters is read
- **THEN** it is returned within 300ms

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
