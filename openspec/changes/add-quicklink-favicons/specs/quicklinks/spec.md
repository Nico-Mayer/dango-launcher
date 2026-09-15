## ADDED Requirements

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
