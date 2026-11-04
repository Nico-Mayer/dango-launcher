## MODIFIED Requirements

### Requirement: No-view commands report their outcome

A command declared as `no-view` SHALL NOT emit a view tree. It SHALL report
success or failure. A success MAY carry a one-line message. On success the
launcher SHALL close if it is open, and a success message SHALL show in the
notice (see `notice`). On failure an open launcher SHALL stay open and show the
message in a failure ticker; when the launcher is hidden, as for a command run
from its hotkey, the message SHALL show in the notice instead (see `notice`).

#### Scenario: No-view command succeeds

- **WHEN** a `no-view` command completes successfully
- **THEN** the launcher hides

#### Scenario: No-view command succeeds with a message

- **WHEN** a `no-view` command completes successfully and reports "Brightness set to 60%"
- **THEN** the launcher hides
- **AND** the notice reads "Brightness set to 60%"

#### Scenario: No-view command fails

- **WHEN** a `no-view` command fails
- **THEN** the launcher stays open and the failure is shown as a failure ticker

#### Scenario: No-view command fails without the launcher

- **WHEN** a `no-view` command run from its hotkey fails
- **THEN** the launcher does not appear and the failure is shown in the notice
