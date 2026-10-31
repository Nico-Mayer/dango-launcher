## MODIFIED Requirements

### Requirement: One version source

The system SHALL keep the application version in exactly one file, the Rust
crate manifest, and every produced artifact MUST report that version: the
window and installer metadata, the bundle file names, the git tag, and the
GitHub release name. No other checked-in file SHALL carry an application
version.

#### Scenario: Version read from the crate manifest

- **WHEN** the crate manifest says `0.3.0` and a release build runs
- **THEN** the installer metadata, the bundle file names, and the tag all carry `0.3.0`, and no other checked-in file needs an edit to agree

#### Scenario: Frontend package version is inert

- **WHEN** the repository is searched for an application version field
- **THEN** only the crate manifest carries one
