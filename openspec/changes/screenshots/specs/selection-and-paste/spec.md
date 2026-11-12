## MODIFIED Requirements

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
