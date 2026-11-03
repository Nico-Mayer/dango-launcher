# clipboard-history Specification

## Purpose

The built-in extension that remembers what the user has copied, so that
something replaced on the clipboard minutes ago can still be got back, while
deliberately never remembering what a password manager put there.

## Requirements

### Requirement: What the user copies is recorded

While Dango is running, the extension SHALL record text, images, and files
placed on the clipboard, whichever application put them there. Files copied
together SHALL be one entry, which holds where they are, not their contents.

#### Scenario: Text is recorded

- **WHEN** the user copies text in any application
- **THEN** that text appears in the history as the newest entry

#### Scenario: Image is recorded

- **WHEN** the user copies an image
- **THEN** it appears in the history as the newest entry

#### Scenario: Files are recorded on macOS

- **WHEN** the user selects two files in Finder and copies them
- **THEN** one entry holding both files appears in the history as the newest
  entry
- **AND** no text entry with their names is recorded

#### Scenario: Files are recorded on Windows

- **WHEN** the user selects two files in File Explorer and copies them
- **THEN** one entry holding both files appears in the history as the newest
  entry

#### Scenario: A folder is recorded like a file

- **WHEN** the user copies a folder in the file manager
- **THEN** an entry holding that folder appears in the history

#### Scenario: Copying the same thing again does not duplicate it

- **WHEN** the user copies something already in the history
- **THEN** the existing entry becomes the newest rather than a second entry being added

#### Scenario: Copying the same files again does not duplicate them

- **WHEN** the user copies the same files, in the same order, as an entry
  already in the history
- **THEN** that entry becomes the newest rather than a second entry being added

#### Scenario: Content types that are not handled are ignored

- **WHEN** the clipboard receives content that is neither text, an image, nor
  files
- **THEN** nothing is recorded
- **AND** the history is left as it was

#### Scenario: Recording does not interfere with copying

- **WHEN** an entry is being recorded
- **THEN** the clipboard's contents are unchanged and available to any application that reads them

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

### Requirement: The selected entry is shown in full

The history SHALL show the selected entry beside the list. A text entry SHALL
be shown as it was copied, with its line breaks and indentation, and never
read as formatting. A long text SHALL show at least its start. An image entry
SHALL be shown whole, scaled to fit the pane. A file entry SHALL show the full
path of each of its files, one per line, in the order they were copied.

#### Scenario: A text entry is shown as copied

- **WHEN** the user selects a text entry that spans several indented lines
- **THEN** the pane shows the text with its line breaks and indentation

#### Scenario: Markup characters stay as typed

- **WHEN** the user selects a text entry containing `# heading` and `*stars*`
- **THEN** the pane shows those characters as typed, not as a heading or emphasis

#### Scenario: An image entry is shown whole

- **WHEN** the user selects an image entry
- **THEN** the pane shows the whole image, scaled to fit, with its aspect ratio kept

#### Scenario: A file entry shows its paths

- **WHEN** the user selects an entry holding two files
- **THEN** the pane shows the full path of each, one per line, in the order
  they were copied

#### Scenario: Moving the selection changes the pane

- **WHEN** the user presses Arrow Down in the history
- **THEN** the pane shows the newly selected entry

### Requirement: An entry shows where and when it was copied

Below the entry, the pane SHALL show metadata in this order, leaving out a pair
that does not apply:

- "Source": the application the entry was copied from, when it is known.
- "Copied": when it was last copied, as "Today, 14:31", "Yesterday, 14:31",
  "27 Oct, 14:31" within the current year, or "27 Oct 2025, 14:31" before it,
  in the user's local time.
- "Type": "Text", "Link", "Email address", "Colour", or "File path" for text,
  "Image" for an image, and "File" for one file or "Files" for several.
- "Lines": the number of lines, for text with more than one line.
- "Characters": the number of characters in the whole text, for text.
- "Size": the width and height in pixels, as "1920 × 1080", for an image.
- "Count": the number of files, for a file entry of more than one file.

Copying something already in the history SHALL update its source and its time.

#### Scenario: Source on Windows

- **WHEN** the user copies text in an application on Windows and selects that
  entry in the history
- **THEN** the pane shows "Source" with that application's name

#### Scenario: Source on macOS with one candidate

- **WHEN** the user copies text on macOS while only one application was
  frontmost around the copy, and selects that entry
- **THEN** the pane shows "Source" with that application's name

#### Scenario: Source on macOS with several candidates

- **WHEN** the user copies text on macOS and switches applications before the
  copy is noticed, so several applications could have made it
- **THEN** the pane shows no "Source" pair for that entry

#### Scenario: Source unknown

- **WHEN** an entry was copied by a program that leaves no owner, such as a
  command-line tool
- **THEN** the pane shows no "Source" pair

#### Scenario: Copied time today

- **WHEN** the user selects an entry copied today at 14:31 local time
- **THEN** the pane shows "Copied" with "Today, 14:31"

#### Scenario: Copied time in an earlier year

- **WHEN** the user selects an entry copied on 27 October of last year at 14:31
- **THEN** the pane shows "Copied" with the date including the year and the time

#### Scenario: A link is recognised

- **WHEN** the user selects a text entry that is a single web address
- **THEN** the pane shows "Type" with "Link"

#### Scenario: A colour is recognised

- **WHEN** the user selects a text entry that is a hex colour such as `#ff8800`
- **THEN** the pane shows "Type" with "Colour"

#### Scenario: Plain text

- **WHEN** the user selects a text entry that is none of the recognised kinds
- **THEN** the pane shows "Type" with "Text"

#### Scenario: Text size

- **WHEN** the user selects a text entry of three lines and 38 characters
- **THEN** the pane shows "Lines" with "3" and "Characters" with "38"

#### Scenario: Long text counts in full

- **WHEN** the user selects a text entry longer than the pane shows
- **THEN** "Characters" counts the whole text

#### Scenario: Image size

- **WHEN** the user selects an image entry 1920 pixels wide and 1080 high
- **THEN** the pane shows "Size" with "1920 × 1080"

#### Scenario: One file

- **WHEN** the user selects a file entry of one file
- **THEN** the pane shows "Type" with "File" and no "Count" pair

#### Scenario: Several files

- **WHEN** the user selects a file entry of three files
- **THEN** the pane shows "Type" with "Files" and "Count" with "3"

#### Scenario: Copying again updates the metadata

- **WHEN** the user copies text in one application, then copies the same text
  in another application
- **THEN** the entry shows the second application as its source and the second
  copy's time

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
- **THEN** the launcher opens without waiting for the recording

#### Scenario: A large image does not stall the launcher

- **WHEN** a large image is being recorded
- **THEN** root search stays responsive to typing

### Requirement: Browsing image entries stays responsive

Clipboard History SHALL respond to key presses without visible lag however
many of its listed entries are images, including large screenshots, and with
the selected image shown in the pane. Leaving the history SHALL give back the
memory its listed and shown images used.

#### Scenario: Moving through many image entries

- **WHEN** the history lists as many image entries as it can show, and the
  user presses Arrow Down several times quickly
- **THEN** the selection follows each press without visible lag
- **AND** the pane shows each newly selected image without visible lag

#### Scenario: Moving through large screenshots

- **WHEN** the history lists several full-screen screenshots and the user moves
  the selection between them
- **THEN** the selection and the pane follow without visible lag

#### Scenario: Typing over many image entries

- **WHEN** the history lists many image entries and the user types to narrow it
- **THEN** each character appears and the list narrows without visible lag

#### Scenario: Thumbnails stay sharp on high-density displays

- **WHEN** an image entry is listed on a display scaled above 100 %
- **THEN** its thumbnail is not visibly blurred or blocky

#### Scenario: The pane image stays sharp on high-density displays

- **WHEN** an image entry larger than the pane is selected on a display scaled
  above 100 %
- **THEN** the image in the pane is not visibly blurred or blocky

#### Scenario: Memory is given back after leaving the history

- **WHEN** the user leaves the history after browsing many image entries
- **THEN** Dango's memory use returns to about what it was before the history
  was opened

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
