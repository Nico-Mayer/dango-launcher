# Spec Delta

## MODIFIED Requirements

### Requirement: Favicon fetching can be turned off

The user SHALL be able to stop Dango making requests to the sites their
quicklinks point at, through the setting `quicklinks.favicons`, a
boolean that defaults to `true`.

#### Scenario: Turning it off

- **WHEN** `favicons` is set to `false`
- **THEN** Dango makes no request to any quicklink's host
- **AND** every quicklink is shown with the launcher's quicklink icon

#### Scenario: Turning it back on

- **WHEN** `favicons` is set back to `true`
- **THEN** fetching resumes without a restart

#### Scenario: The quicklinks extension is disabled

- **WHEN** the `dango.quicklinks` extension is turned off
- **THEN** no favicon is fetched
