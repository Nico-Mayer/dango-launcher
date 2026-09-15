# quicklinks Specification

## Purpose

The built-in extension that turns a URL the user visits often into something
they reach by name from the launcher, including the ones that take a search term
and therefore cannot just be a bookmark.

## Requirements

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

### Requirement: A quicklink carries its site's icon

A quicklink SHALL be shown with the favicon of the site its URL points at
wherever the launcher lists it, so quicklinks are told apart without reading
their titles. A quicklink whose favicon is not available SHALL be shown with the
launcher's quicklink icon. Behaviour is identical on Windows and macOS.

#### Scenario: A quicklink whose favicon is available

- **WHEN** a quicklink for a site whose favicon Dango holds appears in root search
- **THEN** that favicon is drawn as the quicklink's icon

#### Scenario: The same icon in the quicklinks list

- **WHEN** the user opens Search Quicklinks
- **THEN** each quicklink is drawn with the same icon it has in root search

#### Scenario: No favicon yet

- **WHEN** a quicklink's favicon has not been fetched yet, or its site has none
- **THEN** the quicklink is shown with the launcher's quicklink icon
- **AND** it is never shown with the snippet icon or an empty placeholder

#### Scenario: Two quicklinks to the same site

- **WHEN** two quicklinks point at different pages of one site
- **THEN** both are shown with that site's favicon

#### Scenario: A URL that takes a query

- **WHEN** a quicklink's URL template has a placeholder in it
- **THEN** its favicon is the one of the site the template points at, with the
  placeholder left empty

#### Scenario: Showing an icon costs the search path nothing

- **WHEN** root search queries the quicklinks provider with 500 quicklinks stored
- **THEN** it answers within the 50ms provider budget
- **AND** no network request is made while answering

### Requirement: Favicons are fetched in the background and kept

Dango SHALL fetch the favicon for each host its quicklinks point at away from
the search path, keep it on disk, and reuse it across restarts. A fetch that
fails SHALL leave the quicklink usable and SHALL NOT be retried on every
keystroke or every start.

#### Scenario: A new quicklink gets its favicon

- **WHEN** the user creates a quicklink for a host Dango has no favicon for
- **THEN** the favicon is fetched in the background
- **AND** the quicklink shows it once it arrives, without a restart

#### Scenario: Favicons survive a restart

- **WHEN** Dango is stopped and started again
- **THEN** quicklinks whose favicons were fetched still show them
- **AND** those favicons are not fetched again while they are current

#### Scenario: A site with no favicon

- **WHEN** a quicklink's site serves no favicon
- **THEN** the quicklink stays usable with the launcher's quicklink icon
- **AND** the site is not asked again until the retry delay has passed

#### Scenario: A site that cannot be reached

- **WHEN** the machine is offline, or a host does not answer
- **THEN** no error is shown and every quicklink still opens
- **AND** the failure is recorded where a developer can read it

#### Scenario: A response that is not a usable icon

- **WHEN** a host answers with something that is not an image the launcher can
  draw, or with an image larger than the size limit
- **THEN** nothing is cached for that host and the quicklink keeps the generic icon

#### Scenario: A favicon that changed

- **WHEN** a cached favicon is older than the refresh age and its quicklink is
  still there
- **THEN** it is fetched again in the background
- **AND** the quicklink shows the old icon until the new one is written

#### Scenario: A host no longer used

- **WHEN** every quicklink pointing at a host has been removed
- **THEN** that host's favicon is not fetched again

### Requirement: Favicon fetching can be turned off

The user SHALL be able to stop Dango making requests to the sites their
quicklinks point at, through a `dango.quicklinks` preference that defaults to
on.

#### Scenario: Turning it off

- **WHEN** the preference is set to off
- **THEN** Dango makes no request to any quicklink's host
- **AND** every quicklink is shown with the launcher's quicklink icon

#### Scenario: Turning it back on

- **WHEN** the preference is set back to on
- **THEN** fetching resumes without a restart

#### Scenario: The quicklinks extension is disabled

- **WHEN** the `dango.quicklinks` extension is turned off
- **THEN** no favicon is fetched
