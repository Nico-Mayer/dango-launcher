# clipboard-history Specification

## Purpose

The built-in extension that remembers what the user has copied, so that
something replaced on the clipboard minutes ago can still be got back, while
deliberately never remembering what a password manager put there.

## Requirements

### Requirement: What the user copies is recorded

While Dango is running, the extension SHALL record text and images placed on the
clipboard, whichever application put them there.

#### Scenario: Text is recorded

- **WHEN** the user copies text in any application
- **THEN** that text appears in the history as the newest entry

#### Scenario: Image is recorded

- **WHEN** the user copies an image
- **THEN** it appears in the history as the newest entry

#### Scenario: Copying the same thing again does not duplicate it

- **WHEN** the user copies something already in the history
- **THEN** the existing entry becomes the newest rather than a second entry being added

#### Scenario: Content types that are not handled are ignored

- **WHEN** the clipboard receives content that is neither text nor an image
- **THEN** nothing is recorded
- **AND** the history is left as it was

#### Scenario: Recording does not interfere with copying

- **WHEN** an entry is being recorded
- **THEN** the clipboard's contents are unchanged and available to any application that reads them

### Requirement: Excluded content is never recorded

Content marked by the operating system as not for history, and content copied
from an application the user has excluded, SHALL NOT be recorded. An excluded
entry SHALL leave no trace: not in the database, not on disk, and not in memory
beyond the check itself.

Content Dango itself puts on the clipboard SHALL NOT be recorded either, and
SHALL be recognised as its own even when the system does not hand it back
unchanged. Recognition SHALL be bounded in time, so that the user copying the
same thing later is still recorded as theirs.

#### Scenario: Concealed content on macOS

- **WHEN** an application marks clipboard content as concealed, as password managers do
- **THEN** nothing is recorded

#### Scenario: Excluded content on Windows

- **WHEN** an application marks clipboard content as excluded from history, as password managers do
- **THEN** nothing is recorded

#### Scenario: Application on the exclusion list

- **WHEN** content is copied from an application the user has added to the exclusion list
- **THEN** nothing is recorded

#### Scenario: Excluded application left before the copy was noticed

- **WHEN** content is copied from an excluded application and the user switches to another application before the copy is noticed
- **THEN** nothing is recorded, because any application frontmost around the copy is enough to exclude it

#### Scenario: Exclusion list is populated out of the box

- **WHEN** the user has never edited the exclusion list
- **THEN** mainstream password managers are already excluded

#### Scenario: Exclusion list is read at run time

- **WHEN** the user changes the exclusion list while Dango is running
- **THEN** the change takes effect without restarting Dango

#### Scenario: Dango's own copying is not recorded

- **WHEN** Dango puts an entry back on the clipboard
- **THEN** that does not create a new entry or reorder the history

#### Scenario: An image Dango wrote comes back re-encoded

- **WHEN** Dango puts an image on the clipboard and the system hands it back
  encoded differently from what was written
- **THEN** it is still recognised as Dango's own write and no entry is created

#### Scenario: The user copies an image of their own afterwards

- **WHEN** the user copies an image themselves, later than the window in which
  Dango's own write is recognised
- **THEN** it is recorded, even if it is the same image Dango wrote

### Requirement: The history is bounded

The history SHALL keep at most a fixed number of entries and SHALL NOT exceed a
total size on disk. When either bound is reached, the oldest entries SHALL be
discarded first.

#### Scenario: Entry count is capped

- **WHEN** recording an entry would exceed the entry limit
- **THEN** the oldest entry is discarded so the limit holds

#### Scenario: Total size is capped

- **WHEN** recording an entry would take the history over its size ceiling
- **THEN** the oldest entries are discarded until it fits

#### Scenario: A single item too large to keep

- **WHEN** a copied item is larger than the per-entry ceiling
- **THEN** it is not recorded
- **AND** the rest of the history is untouched

#### Scenario: Discarding an image reclaims its space

- **WHEN** an image entry is discarded
- **THEN** the file holding it is deleted

### Requirement: The history is browsable and restorable

The extension SHALL contribute a command that opens the history, newest first,
narrowable by typing. Each entry SHALL offer a paste action, which inserts the
entry into the application the user was in, and a copy action, which places the
entry on the clipboard. Both SHALL hide the launcher as they run.

Which of the two Enter runs SHALL be set by the extension preference
`primary-action`, with the values `paste` and `copy`. The default SHALL be
`paste`. The action Enter does not run SHALL be reachable from the action
panel. A value that is neither SHALL behave as the default.

#### Scenario: History opens newest first

- **WHEN** the user invokes the history command
- **THEN** the entries are listed with the most recently copied first

#### Scenario: List narrows as the user types

- **WHEN** the user types while the history is open
- **THEN** the list narrows to the entries whose text matches

#### Scenario: Enter pastes by default

- **WHEN** `primary-action` is not set and the user presses Enter on a text
  entry while an editable field is focused in another application
- **THEN** the launcher hides
- **AND** the entry's text appears in that field
- **AND** the clipboard holds the entry once the paste has finished

#### Scenario: Pasting an image entry

- **WHEN** the user pastes an image entry into an application that accepts
  pasted images
- **THEN** the launcher hides
- **AND** the image is pasted into that application
- **AND** the clipboard holds the image once the paste has finished

#### Scenario: Choosing an entry restores it

- **WHEN** the user chooses the copy action on an entry
- **THEN** its content is placed on the clipboard
- **AND** the launcher hides

#### Scenario: Enter copies when configured

- **WHEN** `primary-action` is `copy` and the user presses Enter on an entry
- **THEN** its content is placed on the clipboard
- **AND** the launcher hides
- **AND** nothing is sent to any other application

#### Scenario: The other action is in the panel

- **WHEN** the user opens the action panel on an entry
- **THEN** the action Enter does not run is listed there, along with remove
- **AND** choosing it runs it

#### Scenario: The preference is read live

- **WHEN** the user changes `primary-action` in the configuration file while
  Dango is running and opens the history again
- **THEN** Enter runs the newly configured action, without a restart

#### Scenario: An unrecognised preference value

- **WHEN** `primary-action` is set to a value other than `paste` or `copy`
- **THEN** Enter pastes, as if the preference were not set

#### Scenario: The launcher gets out of the way promptly

- **WHEN** the user confirms paste or copy on an entry
- **THEN** the launcher is no longer on screen within 100ms of the key press

#### Scenario: Pasting is not possible on macOS without the permission

- **WHEN** the user pastes an entry and the Accessibility permission has not
  been granted
- **THEN** nothing is sent to the other application
- **AND** the launcher stays open and explains that the permission is needed
- **AND** the clipboard is unchanged

#### Scenario: Pasting does not reorder the history

- **WHEN** the user pastes an entry that is not the newest
- **THEN** the history keeps its order
- **AND** no new entry is recorded for the paste

#### Scenario: Pasting on Windows needs no permission

- **WHEN** the user pastes an entry on Windows
- **THEN** it is inserted with no permission step

#### Scenario: Pasting is not possible for another reason

- **WHEN** the entry cannot be inserted, such as when there is no application
  to return to
- **THEN** the launcher stays open with a message explaining why
- **AND** nothing has been pasted

#### Scenario: Text entries are recognisable in the list

- **WHEN** a text entry is listed
- **THEN** it is shown on one line, with leading whitespace and line breaks collapsed so it can be told apart at a glance

#### Scenario: Image entries are recognisable in the list

- **WHEN** an image entry is listed
- **THEN** it is shown with a thumbnail of itself

#### Scenario: Empty history

- **WHEN** the user invokes the history command having copied nothing
- **THEN** an empty state explains that nothing has been copied yet

#### Scenario: An entry can be removed

- **WHEN** the user chooses the remove action on an entry
- **THEN** it is deleted from the history and its file, if it has one, is deleted
- **AND** the list stays open

#### Scenario: Entry whose file has gone

- **WHEN** the user chooses an image entry whose file is missing
- **THEN** the failure is reported
- **AND** the entry is removed from the history

### Requirement: The history survives a restart

Recorded entries SHALL still be there after Dango restarts.

#### Scenario: History persists

- **WHEN** Dango restarts
- **THEN** the entries recorded before it stopped are still listed, in the same order

#### Scenario: Nothing is recorded while Dango is not running

- **WHEN** the user copies something while Dango is not running
- **THEN** it is absent from the history, because nothing was watching

### Requirement: Watching does not cost the launcher its responsiveness

The watcher SHALL run off the activation path and SHALL NOT delay the launcher
opening or the result list appearing.

#### Scenario: Activation during recording

- **WHEN** the user opens the launcher while an entry is being recorded
- **THEN** the launcher opens within its activation budget of 80ms

#### Scenario: A large image does not stall the launcher

- **WHEN** a large image is being recorded
- **THEN** root search stays responsive to typing
