## MODIFIED Requirements

### Requirement: The selected text can be read

Dango SHALL be able to read the text currently selected in the frontmost
application, and SHALL report the absence of a selection distinctly from a
failure to read one.

On both platforms Dango SHALL first ask the application directly, and SHALL
send no keystrokes to do it. The clipboard round trip SHALL be a fallback, used
only where the application cannot be asked.

An application that answers but reports nothing selected SHALL be treated as an
empty selection, not as a failure to read, and SHALL NOT fall through to the
keystroke path. The direct request SHALL be bounded in time, so an application
that does not answer cannot hold up the read.

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

- **WHEN** the frontmost application exposes its text but nothing is selected,
  on either platform
- **THEN** an empty selection is reported
- **AND** no keystroke is sent

#### Scenario: The application will not say what is selected

- **WHEN** the frontmost application cannot be asked what is selected
- **THEN** Dango falls back to reading it through the clipboard
- **AND** the clipboard holds what it held before the read, once the read has
  finished

#### Scenario: Reading a selection is fast enough to act on

- **WHEN** a selection of 1,000 characters is read
- **THEN** it is returned without visible delay

#### Scenario: An application that does not answer in time

- **WHEN** the frontmost application has not answered the direct request after
  a short bounded wait, on either platform
- **THEN** Dango stops waiting for it
- **AND** the read completes by the fallback or reports that it could not be done

### Requirement: Dango's own clipboard use never reaches the history

No clipboard write Dango makes, to read a selection, to insert text, or to put
a clipboard history entry back on the clipboard, SHALL appear in the clipboard
history, and no such write SHALL reorder an entry that is already there.

A write SHALL be recognised as Dango's own even when the system does not hand
it back unchanged. Recognition SHALL be bounded in time, so that the user
copying the same thing later is still recorded as theirs.

#### Scenario: Inserted text is not recorded

- **WHEN** Dango inserts text while the clipboard history is recording
- **THEN** that text does not appear in the history

#### Scenario: Restoring the clipboard does not reorder the history

- **WHEN** Dango restores the user's clipboard after inserting text
- **THEN** the history is in the same order it was in before

#### Scenario: Reading a selection through the clipboard is not recorded

- **WHEN** Dango reads a selection by copying it
- **THEN** the copied text does not appear in the history

#### Scenario: Putting a history entry back is not recorded

- **WHEN** Dango puts a clipboard history entry back on the clipboard
- **THEN** that does not create a new entry or reorder the history

#### Scenario: An image Dango wrote comes back re-encoded

- **WHEN** Dango puts an image on the clipboard and the system hands it back
  encoded differently from what was written
- **THEN** it is still recognised as Dango's own write and no entry is created

#### Scenario: The user copies an image of their own afterwards

- **WHEN** the user copies an image themselves, later than the window in which
  Dango's own write is recognised
- **THEN** it is recorded, even if it is the same image Dango wrote

#### Scenario: A genuine copy during the operation is still recorded

- **WHEN** the user copies something themselves while Dango is inserting text
- **THEN** what they copied is recorded in the history

### Requirement: Failures are reported, never silent

An insertion or a read that cannot be completed SHALL leave a message the user
can act on, and SHALL leave the clipboard as it found it. The message SHALL name
the actual cause: a missing permission SHALL be reported only when the
permission is missing.

The text path SHALL NOT depend on the clipboard history. With the history
unavailable, reading the selection and inserting text SHALL still work.

#### Scenario: The target window is elevated on Windows

- **WHEN** the frontmost window belongs to an elevated process
- **THEN** the insertion fails with a message saying so
- **AND** the clipboard is unchanged

#### Scenario: There is no application to insert into

- **WHEN** there is no previous application to return to
- **THEN** the insertion fails with a message saying so
- **AND** the clipboard is unchanged

#### Scenario: The text path could not be set up on Windows

- **WHEN** Dango could not set up the text path at startup on Windows and the user invokes something that reads the selection or inserts text
- **THEN** the launcher shows "Dango can't reach other apps right now"
- **AND** the message does not mention a permission

#### Scenario: The text path could not be set up on macOS

- **WHEN** Dango could not set up the text path at startup on macOS and the user invokes something that reads the selection or inserts text
- **THEN** the launcher shows "Dango can't reach other apps right now"
- **AND** the message does not mention a permission

#### Scenario: The tray reports a text path that could not be set up

- **WHEN** Dango could not set up the text path at startup
- **THEN** a disabled tray item reads "Pasting is off, see dango.log"

#### Scenario: The clipboard history is unavailable

- **WHEN** the database could not be opened at startup
- **THEN** clipboard history is off
- **AND** reading the selection and inserting text still work, through AI commands and snippets

### Requirement: The keystroke fallback can be refused for an application

The user SHALL be able to name applications where the clipboard fallback must
not be attempted, because the copy chord does something else there. For a
named application Dango SHALL report that it cannot read the selection, and
SHALL send no keystroke.

The applications SHALL be set by the config key
`selection.excluded-applications`: application names, comma separated, matched
ignoring case, the same way as the clipboard history's exclusion list. It SHALL
be empty by default.

#### Scenario: A named application is never sent the copy chord

- **WHEN** the frontmost application is one the user named, and something reads
  the selection
- **THEN** no keystroke is sent to it
- **AND** the failure says to copy the text first, as that application does not
  share its selection

#### Scenario: Names match ignoring case

- **WHEN** `selection.excluded-applications` names "helix" and the frontmost
  application is named "Helix"
- **THEN** it is treated as a named application

#### Scenario: A named application that does expose its selection still works

- **WHEN** a named application exposes its selection directly
- **THEN** the selection is read directly
- **AND** the exclusion never comes into play

#### Scenario: An application that is not named is unaffected

- **WHEN** the frontmost application is not named and does not expose its
  selection
- **THEN** the clipboard fallback runs

#### Scenario: The list is empty by default

- **WHEN** `selection.excluded-applications` is not set
- **THEN** every application that does not expose its selection gets the
  clipboard fallback

## REMOVED Requirements

### Requirement: The macOS permission is a visible state

**Reason**: Its scenarios for requesting the permission and noticing a grant
repeated `openspec/specs/permissions`.

**Migration**: Replaced by "Reading and inserting need Accessibility on
macOS", which keeps the failure rule and links to permissions for the rest.

## ADDED Requirements

### Requirement: Reading and inserting need Accessibility on macOS

On macOS both reading the selection and inserting text require the Accessibility
permission. Dango SHALL detect that it is missing and SHALL explain it, rather
than failing silently. Requesting the permission and noticing a grant are in
`openspec/specs/permissions`.

#### Scenario: Acting without the permission on macOS

- **WHEN** the user invokes something that reads the selection or inserts text and the permission has not been granted
- **THEN** nothing is sent to the other application
- **AND** the launcher shows "Allow Accessibility in System Settings"

#### Scenario: The permission is granted on macOS

- **WHEN** the permission has been granted
- **THEN** reading the selection and inserting text work without any further prompting

#### Scenario: Windows needs no permission

- **WHEN** the user invokes something that reads the selection or inserts text on Windows
- **THEN** it works with no permission step
- **AND** no message about a permission is ever shown
