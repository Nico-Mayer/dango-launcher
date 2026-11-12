## MODIFIED Requirements

### Requirement: Marked and excluded content is never recorded

Content marked by the operating system as not for history, and content copied
from an application the user has excluded, SHALL NOT be recorded. An excluded
entry SHALL leave no trace: not in the database, not on disk, and not in memory
beyond the check itself. This SHALL hold for text, images, and files alike.

The excluded applications SHALL be set by the setting `excluded-applications`
in `[clipboard]`: a list of application names, matched ignoring case. Its
default SHALL list mainstream password managers.

Content Dango itself puts on the clipboard SHALL NOT be recorded either,
except a copy the user asked for, such as a Copy action or a pasted
screenshot. Both rules are in `openspec/specs/selection-and-paste`.

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
