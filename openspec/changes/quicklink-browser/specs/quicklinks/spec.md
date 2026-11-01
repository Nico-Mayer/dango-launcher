## ADDED Requirements

### Requirement: A quicklink can name the browser it opens in

A quicklink SHALL have an optional browser, chosen in its create and edit form
from a fixed list. The list SHALL start with "Default browser", then offer
Google Chrome, Microsoft Edge, Firefox, and Brave on both platforms, and Safari
on macOS only. The list SHALL be the same whether or not a browser is
installed. A quicklink without a browser SHALL open in the default browser.

#### Scenario: The form offers the browsers on Windows

- **WHEN** the user opens the create form for a quicklink on Windows
- **THEN** the "Browser" field offers "Default browser", "Google Chrome",
  "Microsoft Edge", "Firefox", and "Brave", in that order
- **AND** "Default browser" is selected

#### Scenario: The form offers the browsers on macOS

- **WHEN** the user opens the create form for a quicklink on macOS
- **THEN** the "Browser" field offers "Default browser", "Google Chrome",
  "Microsoft Edge", "Firefox", "Brave", and "Safari", in that order
- **AND** "Default browser" is selected

#### Scenario: Choosing a browser

- **WHEN** the user picks a browser in the form and saves the quicklink
- **THEN** the stored quicklink names that browser
- **AND** the edit form shows that browser selected

#### Scenario: Going back to the default browser

- **WHEN** the user edits a quicklink that names a browser, picks
  "Default browser", and saves
- **THEN** the stored quicklink no longer names a browser

#### Scenario: A browser not offered here survives an edit

- **WHEN** the user edits a quicklink whose browser is not offered on this
  platform, such as Safari on Windows, and saves without changing the browser
- **THEN** the form shows "Default browser" selected
- **AND** the stored quicklink still names its browser

## MODIFIED Requirements

### Requirement: Confirming a quicklink opens it

Confirming a quicklink SHALL render its URL template and open the result in the
quicklink's browser, or in the user's default browser when the quicklink names
none. When the named browser cannot be used, because it is not installed or not
offered on this platform, the URL SHALL open in the default browser without a
message.

#### Scenario: A quicklink with no query

- **WHEN** the user confirms a quicklink whose template takes no query and that
  names no browser
- **THEN** the launcher closes and the URL opens in the default browser

#### Scenario: A quicklink with a browser

- **WHEN** the user confirms a quicklink that names an installed browser
- **THEN** the launcher closes and the URL opens in that browser

#### Scenario: The named browser is not installed

- **WHEN** the user confirms a quicklink that names a browser not installed on
  this machine
- **THEN** the launcher closes and the URL opens in the default browser
- **AND** no message is shown

#### Scenario: Safari on Windows

- **WHEN** the user confirms a quicklink that names Safari on Windows
- **THEN** the launcher closes and the URL opens in the default browser
- **AND** no message is shown

#### Scenario: A quicklink that could not be opened

- **WHEN** the rendered URL cannot be opened in the named browser nor in the
  default browser
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
- **THEN** the URL is rendered with that query and opened the same way as a
  quicklink with no query, in its browser or the default browser

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

### Requirement: Quicklinks survive a restart

A quicklink SHALL persist across restarts, unchanged in name, URL template, and
browser.

#### Scenario: Quicklinks come back after a restart

- **WHEN** Dango is stopped and started again
- **THEN** every quicklink the user created is still there with the same name,
  URL template, and browser

#### Scenario: A removed quicklink stays removed

- **WHEN** a quicklink is removed and Dango is restarted
- **THEN** it does not come back
