## MODIFIED Requirements

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
