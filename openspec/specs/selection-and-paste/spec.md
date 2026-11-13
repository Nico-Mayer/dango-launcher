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

On both platforms Dango SHALL first ask the application directly, and SHALL
send no keystrokes to do it. The clipboard round trip SHALL be a fallback, used
only where the application cannot be asked.

An application that answers but reports nothing selected SHALL be treated as an
empty selection, not as a failure to read, and SHALL NOT fall through to the
keystroke path. An answer from Dango's own window, or an empty answer from an
element that exposes no selection at all, SHALL count as no answer. The direct
request SHALL be bounded in time, so an application that does not answer
cannot hold up the read.

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

#### Scenario: Dango's own search field is never the answer on macOS

- **WHEN** the launcher is on screen on macOS, the application behind it
  exposes nothing, and Dango reads the selection
- **THEN** the empty search field of the launcher is not reported as an empty
  selection
- **AND** Dango falls back to reading it through the clipboard

#### Scenario: An element that exposes no selection on macOS

- **WHEN** the focused element of the frontmost application on macOS answers
  with empty selected text but exposes no selected text range
- **THEN** Dango falls back to reading it through the clipboard

#### Scenario: Reading a selection is fast enough to act on

- **WHEN** a selection of 1,000 characters is read
- **THEN** it is returned without visible delay

#### Scenario: An application that does not answer in time

- **WHEN** the frontmost application has not answered the direct request after
  a short bounded wait, on either platform
- **THEN** Dango stops waiting for it
- **AND** the read completes by the fallback or reports that it could not be done

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
- **THEN** the text appears in the target application without visible delay

#### Scenario: The caret can be placed inside the inserted text

- **WHEN** text is inserted that declares where the caret should end up
- **THEN** the whole text is inserted
- **AND** the caret is left at the declared position rather than at the end

### Requirement: The user's clipboard survives

Reading a selection and inserting text MAY use the clipboard, and the user SHALL
NOT be able to observe that it did. The clipboard's contents SHALL be the same
before and after, for text and for images alike.

The exceptions are pasting an entry from the clipboard history and pasting a
screenshot. There the user has chosen content that lives on the clipboard, so
the entry or screenshot SHALL be left on the clipboard after it is pasted,
ready to be pasted again.

#### Scenario: The clipboard is restored after an insertion

- **WHEN** the user has content on the clipboard and Dango inserts text
- **THEN** the clipboard holds that same content once the insertion has finished

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

#### Scenario: A pasted screenshot stays on the clipboard

- **WHEN** the user pastes a screenshot
- **THEN** the clipboard holds that screenshot once the paste has finished
- **AND** pasting again in the application pastes the same screenshot

### Requirement: Dango's own clipboard use never reaches the history

No clipboard write Dango makes, to read a selection, to insert text, or to put
a clipboard history entry back on the clipboard, SHALL appear in the clipboard
history, and no such write SHALL reorder an entry that is already there.

A copy the user asks for is theirs, not Dango's. What a Copy action places on
the clipboard, and a screenshot Dango puts there to copy or paste it, SHALL be
recorded like a copy the user made in another application. A screenshot SHALL
be recorded as a file entry, like the user copying the file in the file
manager.

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

#### Scenario: A copied path is recorded

- **WHEN** the user runs "Copy path" on an application while the clipboard history is recording
- **THEN** the history's newest entry is that path

#### Scenario: A copied screenshot is recorded

- **WHEN** the user copies or pastes a screenshot while the clipboard history is recording
- **THEN** the history's newest entry is a file entry holding that screenshot's file

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

### Requirement: Reading the selection keeps the launcher on screen

Reading the selection SHALL NOT hide the launcher, on either platform, whether
the command was run from root search or from its own hotkey. When the read is
done, keyboard input SHALL go to the launcher again, to the field that had it
before. A failed read SHALL be shown in the launcher at once.

#### Scenario: Reading on Windows

- **WHEN** the launcher is on screen on Windows and the user runs a command
  that reads the selection
- **THEN** the launcher stays on screen during the read
- **AND** once the read is done, typing goes to the launcher

#### Scenario: Reading on macOS

- **WHEN** the launcher is on screen on macOS and the user runs a command that
  reads the selection
- **THEN** the launcher stays on screen during the read
- **AND** typing still goes to the launcher

#### Scenario: The copy fallback on Windows

- **WHEN** the application behind the launcher on Windows exposes nothing and
  the selection is read through the clipboard
- **THEN** the launcher stays on screen
- **AND** the copy reaches the application behind it, not the launcher

#### Scenario: The copy fallback on macOS

- **WHEN** the application behind the launcher on macOS exposes nothing and
  the selection is read through the clipboard
- **THEN** the launcher stays on screen
- **AND** the copy reaches the application behind it, not the launcher

#### Scenario: A failed read is shown at once

- **WHEN** a command reads the selection and nothing is selected
- **THEN** the launcher is still on screen and shows "Select some text first"

#### Scenario: Lending the foreground is not a focus loss on Windows

- **WHEN** Dango gives the foreground to the previous window to read the
  selection
- **THEN** the launcher is not hidden as if the user had clicked away

### Requirement: The copy fallback uses a chord that copies everywhere

The keystroke Dango sends to read a selection through the clipboard SHALL be
one that means copy in editors and terminals alike, and SHALL NOT interrupt a
running program or run an editor command. On Windows it SHALL be Ctrl+Insert.
On macOS it SHALL be Cmd+C.

#### Scenario: The copy chord on Windows

- **WHEN** Dango reads a selection through the clipboard on Windows
- **THEN** it sends Ctrl+Insert and never Ctrl+C

#### Scenario: An editor that copies on Ctrl+Insert

- **WHEN** text is selected in Zed on Windows and an AI command that needs the
  selection is run
- **THEN** the selected text is sent with the command's prompt

#### Scenario: The copy chord on macOS

- **WHEN** Dango reads a selection through the clipboard on macOS
- **THEN** it sends Cmd+C to the application behind the launcher
