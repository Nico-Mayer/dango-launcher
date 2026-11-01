## MODIFIED Requirements

### Requirement: The history is bounded

The history SHALL keep at most a set number of entries and SHALL NOT exceed a
total size on disk. When either bound is reached, the oldest entries SHALL be
discarded first. A single entry larger than a per-entry ceiling SHALL NOT be
recorded.

The bounds SHALL be set by extension preferences: `entries`, the number of
entries, default 500; `entry-megabytes`, the per-entry ceiling, default 16;
and `total-megabytes`, the total size, default 256.

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

- **WHEN** the user has set none of the three preferences
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

#### Scenario: Pasting an image entry

- **WHEN** the user pastes an image entry into an application that accepts
  pasted images
- **THEN** the launcher hides
- **AND** the image is pasted into that application

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

## REMOVED Requirements

### Requirement: Excluded content is never recorded

**Reason**: Its rule and scenarios on Dango's own clipboard writes repeated
`openspec/specs/selection-and-paste`, which now holds the single copy.

**Migration**: Replaced by "Marked and excluded content is never recorded".
The own-write scenarios are in selection-and-paste under "Dango's own
clipboard use never reaches the history".

## ADDED Requirements

### Requirement: Marked and excluded content is never recorded

Content marked by the operating system as not for history, and content copied
from an application the user has excluded, SHALL NOT be recorded. An excluded
entry SHALL leave no trace: not in the database, not on disk, and not in memory
beyond the check itself.

The excluded applications SHALL be set by the extension preference
`excluded-applications`: application names, comma separated, matched ignoring
case. Its default SHALL list mainstream password managers.

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

#### Scenario: Exclusion matches names ignoring case

- **WHEN** `excluded-applications` names "1password" and content is copied from
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
