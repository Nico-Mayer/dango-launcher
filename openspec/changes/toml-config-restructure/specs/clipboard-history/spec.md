# Spec Delta

## MODIFIED Requirements

### Requirement: The history is bounded

The history SHALL keep at most a set number of entries and SHALL NOT exceed a
total size on disk. When either bound is reached, the oldest entries SHALL be
discarded first. A single entry larger than a per-entry ceiling SHALL NOT be
recorded.

The bounds SHALL be set by whole-number settings in `[clipboard]`: `entries`,
the number of entries, default 500; `entry-megabytes`, the per-entry ceiling,
default 16; and `total-megabytes`, the total size, default 256.

#### Scenario: Entry count is capped

- **WHEN** recording an entry would exceed `entries`
- **THEN** the oldest entry is discarded so the limit holds

#### Scenario: Total size is capped

- **WHEN** recording an entry would take the history over `total-megabytes`
- **THEN** the oldest entries are discarded until it fits

#### Scenario: A single item too large to keep

- **WHEN** a copied item is larger than `entry-megabytes`
- **THEN** it is not recorded
- **AND** the rest of the history is untouched

#### Scenario: Defaults apply when nothing is set

- **WHEN** the user has set none of the three settings
- **THEN** the history keeps at most 500 entries, refuses an entry over 16
  megabytes, and stays within 256 megabytes

#### Scenario: Discarding an image reclaims its space

- **WHEN** an image entry is discarded
- **THEN** the file holding it is deleted

### Requirement: The history is browsable and restorable

The extension SHALL contribute a command that opens the history, newest first,
narrowable by typing. Each entry SHALL offer a paste action, which inserts the
entry into the application the user was in, and a copy action, which places the
entry on the clipboard. Both SHALL hide the launcher as they run. What the
clipboard holds after a paste is in `openspec/specs/selection-and-paste`.

Which of the two Enter runs SHALL be set by the setting `primary-action` in
`[clipboard]`, with the values `paste` and `copy`. The default SHALL be
`paste`. The action Enter does not run SHALL be reachable from the action
panel. A value that is neither SHALL make the config file invalid, as
described in `openspec/specs/configuration`.

A file entry SHALL go back on the clipboard as files, holding those of its
files that still exist.

#### Scenario: History opens newest first

- **WHEN** the user invokes the history command
- **THEN** the entries are listed with the most recently copied first

#### Scenario: List narrows as the user types

- **WHEN** the user types while the history is open
- **THEN** the list narrows to the entries whose text matches

#### Scenario: List narrows to file names

- **WHEN** the history holds an entry of three files and the user types part of
  the third file's name
- **THEN** that entry stays in the list

#### Scenario: Enter pastes by default

- **WHEN** `primary-action` is not set and the user presses Enter on a text
  entry while an editable field is focused in another application
- **THEN** the launcher hides
- **AND** the entry's text appears in that field

#### Scenario: Pasting an image entry

- **WHEN** the user pastes an image entry into an application that accepts
  pasted images
- **THEN** the launcher hides
- **AND** the image is pasted into that application

#### Scenario: Pasting a file entry on macOS

- **WHEN** the user pastes a file entry while a Finder window is in front
- **THEN** the launcher hides
- **AND** Finder copies those files into that folder

#### Scenario: Pasting a file entry on Windows

- **WHEN** the user pastes a file entry while a File Explorer window is in front
- **THEN** the launcher hides
- **AND** File Explorer copies those files into that folder

#### Scenario: Copying a file entry

- **WHEN** the user chooses the copy action on a file entry
- **THEN** the clipboard holds those files, so pasting in the file manager
  copies them

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

- **WHEN** the user saves `primary-action` set to a value other than `paste`
  or `copy` while Dango is running
- **THEN** Enter keeps running the action from the last config that loaded
- **AND** the config problem is surfaced

#### Scenario: The launcher gets out of the way promptly

- **WHEN** the user confirms paste or copy on an entry
- **THEN** the launcher leaves the screen at once, without waiting for the
  paste or copy to finish

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

#### Scenario: A file entry of one file is recognisable in the list

- **WHEN** an entry holding only `report.pdf` is listed
- **THEN** it is shown as "report.pdf", with the icon the system shows for that
  file

#### Scenario: A file entry of several files is recognisable in the list

- **WHEN** an entry holding `report.pdf` and two more files is listed
- **THEN** it is shown as "report.pdf and 2 more", with the icon the system
  shows for `report.pdf`

#### Scenario: Empty history

- **WHEN** the user invokes the history command having copied nothing
- **THEN** an empty state reads "Nothing copied yet" with "Text, images, and
  files you copy will show up here"

#### Scenario: An entry can be removed

- **WHEN** the user chooses the remove action on an entry
- **THEN** it is deleted from the history and its file, if it has one, is deleted
- **AND** the list stays open

#### Scenario: Removing a file entry leaves the files alone

- **WHEN** the user removes a file entry
- **THEN** the files it held are not deleted, moved, or changed

#### Scenario: Entry whose file has gone

- **WHEN** the user chooses an image entry whose file is missing
- **THEN** the failure is reported
- **AND** the entry is removed from the history

#### Scenario: Some of a file entry's files have gone

- **WHEN** the user pastes or copies a file entry of three files and one of
  them was deleted
- **THEN** the two that still exist go on the clipboard
- **AND** no failure is reported

#### Scenario: All of a file entry's files have gone

- **WHEN** the user pastes or copies a file entry whose files were all deleted
  or moved
- **THEN** the launcher stays open and reports "The copied files no longer
  exist", or "The copied file no longer exists" for an entry of one file
- **AND** the entry is removed from the history
- **AND** the clipboard is unchanged

### Requirement: Marked and excluded content is never recorded

Content marked by the operating system as not for history, and content copied
from an application the user has excluded, SHALL NOT be recorded. An excluded
entry SHALL leave no trace: not in the database, not on disk, and not in memory
beyond the check itself. This SHALL hold for text, images, and files alike.

The excluded applications SHALL be set by the setting `excluded-applications`
in `[clipboard]`: a list of application names, matched ignoring case. Its
default SHALL list mainstream password managers.

Content Dango itself puts on the clipboard SHALL NOT be recorded either. That
rule is in `openspec/specs/selection-and-paste`.

#### Scenario: Concealed content on macOS

- **WHEN** an application marks clipboard content as concealed, as password managers do
- **THEN** nothing is recorded

#### Scenario: Excluded content on Windows

- **WHEN** an application marks clipboard content as excluded from history, as password managers do
- **THEN** nothing is recorded

#### Scenario: Application on the exclusion list

- **WHEN** content is copied from an application named in `excluded-applications`
- **THEN** nothing is recorded

#### Scenario: Files from an excluded application

- **WHEN** files are copied from an application named in
  `excluded-applications`
- **THEN** nothing is recorded

#### Scenario: Exclusion matches names ignoring case

- **WHEN** `excluded-applications` lists "1password" and content is copied from
  an application named "1Password"
- **THEN** nothing is recorded

#### Scenario: Excluded application left before the copy was noticed

- **WHEN** content is copied from an excluded application and the user switches to another application before the copy is noticed
- **THEN** nothing is recorded, because any application frontmost around the copy is enough to exclude it

#### Scenario: Exclusion list is populated out of the box

- **WHEN** the user has never set `excluded-applications`
- **THEN** mainstream password managers are already excluded

#### Scenario: Exclusion list is read at run time

- **WHEN** the user changes the exclusion list while Dango is running
- **THEN** the change takes effect without restarting Dango
