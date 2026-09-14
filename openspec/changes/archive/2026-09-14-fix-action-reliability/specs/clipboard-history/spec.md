## MODIFIED Requirements

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
