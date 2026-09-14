## Purpose

The built-in extension that turns a URL the user visits often into something
they reach by name from the launcher, including the ones that take a search term
and therefore cannot just be a bookmark.

## ADDED Requirements

### Requirement: A quicklink is created, edited, and removed from the launcher

The extension SHALL let the user create a quicklink with a name and a URL
template, change either afterwards, and delete it, without leaving the launcher.

#### Scenario: Creating a quicklink

- **WHEN** the user completes the create form with a name and a URL template
- **THEN** the quicklink is stored
- **AND** it can be found in root search straight away

#### Scenario: A quicklink needs a name and a URL

- **WHEN** the user submits the create form with either one empty
- **THEN** the quicklink is not stored
- **AND** the form says what is missing

#### Scenario: A URL that is not a URL is refused

- **WHEN** the user saves a quicklink whose template cannot form a valid URL
- **THEN** it is refused with a message and nothing is stored

#### Scenario: Editing a quicklink

- **WHEN** the user edits a quicklink's name or URL template and submits
- **THEN** the stored quicklink reflects the change

#### Scenario: The form shows what the quicklink will ask for

- **WHEN** the user is writing a quicklink's URL template
- **THEN** the form shows the arguments it will ask for, updating as the template is edited

#### Scenario: Removing a quicklink

- **WHEN** the user removes a quicklink
- **THEN** it no longer appears in root search
- **AND** the launcher surface the user was looking at stays open
- **AND** a root-search query remains unchanged

### Requirement: Quicklinks are found in root search

Every quicklink the user has created SHALL appear as a result in root search,
matched on its name.

#### Scenario: A quicklink is found by its name

- **WHEN** the user types part of a quicklink's name
- **THEN** that quicklink appears in the results

#### Scenario: A quicklink shows where it goes

- **WHEN** a quicklink appears in the results
- **THEN** its name is shown along with an indication of its destination

#### Scenario: Quicklinks stay inside the provider budget

- **WHEN** root search queries the quicklinks provider with 500 quicklinks stored
- **THEN** it answers within the 50ms provider budget

### Requirement: Confirming a quicklink opens it

Confirming a quicklink SHALL render its URL template and open the result in the
user's default browser.

#### Scenario: A quicklink with no query

- **WHEN** the user confirms a quicklink whose template takes no query
- **THEN** the launcher closes and the URL opens in the default browser

#### Scenario: A quicklink that could not be opened

- **WHEN** the rendered URL cannot be opened
- **THEN** the launcher stays open with a message explaining why

### Requirement: A quicklink that takes a query asks for it

A quicklink whose template uses `query` SHALL ask the user for that value before
opening, and SHALL place it into the URL encoded so it cannot change the URL's
meaning.

#### Scenario: The user is asked for the query

- **WHEN** the user confirms a quicklink whose template uses `query`
- **THEN** a form asking for it is shown

#### Scenario: Submitting the query opens the URL

- **WHEN** the user enters a query and submits
- **THEN** the URL is rendered with that query and opened in the default browser

#### Scenario: A query with spaces and reserved characters

- **WHEN** the user enters a query containing spaces or characters such as `&` or `?`
- **THEN** they are encoded in the opened URL
- **AND** the URL's own structure is unchanged

#### Scenario: Abandoning the form opens nothing

- **WHEN** the user dismisses the form without submitting
- **THEN** no URL is opened

#### Scenario: A quicklink using other placeholders

- **WHEN** a quicklink's template uses `clipboard` or `selection`
- **THEN** those values fill themselves in and the user is not asked for them

### Requirement: A quicklink's URL can be copied instead of opened

Every quicklink SHALL offer copying its rendered URL to the clipboard as an
alternative to opening it.

#### Scenario: Copying a quicklink's URL

- **WHEN** the user chooses to copy a quicklink rather than open it
- **THEN** the rendered URL is on the clipboard
- **AND** the launcher closes

### Requirement: Quicklinks survive a restart

A quicklink SHALL persist across restarts, unchanged in name and URL template.

#### Scenario: Quicklinks come back after a restart

- **WHEN** Dango is stopped and started again
- **THEN** every quicklink the user created is still there with the same name and URL template

#### Scenario: A removed quicklink stays removed

- **WHEN** a quicklink is removed and Dango is restarted
- **THEN** it does not come back
