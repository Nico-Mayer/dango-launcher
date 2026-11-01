## MODIFIED Requirements

### Requirement: Next version derives from commit history

The system SHALL compute the next version from the conventional commit messages
since the most recent release tag: a `fix` bumps patch, a `feat` bumps minor,
and a `!` marker or `BREAKING CHANGE` footer bumps major. Commits of other types
MUST NOT bump the version on their own. The release command SHALL always use
the derived version. To pick the version, the developer SHALL call cargo-release
directly with an explicit level or version.

#### Scenario: Feature since last tag

- **WHEN** the last tag is `v0.2.0` and the commits since include `feat(search): ...` and `docs: ...`
- **THEN** the release command produces `v0.3.0`

#### Scenario: Only fixes since last tag

- **WHEN** the last tag is `v0.2.0` and every commit since is `fix:` or a non-bumping type
- **THEN** the release command produces `v0.2.1`

#### Scenario: Nothing to release

- **WHEN** no commit since the last tag is of a bumping type
- **THEN** the release command exits non-zero, says there is nothing to release, and creates no commit or tag

#### Scenario: First release

- **WHEN** the repository has no release tag yet and the crate manifest says `0.1.0`
- **THEN** the release command produces `v0.1.0`, or the developer names the first version by calling cargo-release directly

#### Scenario: Manual override

- **WHEN** the developer calls cargo-release directly with an explicit version or level
- **THEN** that value is used instead of the derived one, and the rest of the release proceeds the same way

## ADDED Requirements

### Requirement: One application version source

The system SHALL keep the application version in exactly one file, the Rust
crate manifest, and every produced artifact MUST report that version: the
executable and installer metadata, the bundle file names, the git tag, and the
GitHub release name. No other checked-in file SHALL carry an application
version.

#### Scenario: Version read from the crate manifest

- **WHEN** the crate manifest says `0.3.0` and a release build runs
- **THEN** the installer metadata, the bundle file names, and the tag all carry `0.3.0`, and no other checked-in file needs an edit to agree

#### Scenario: No second version field

- **WHEN** the repository is searched for an application version field
- **THEN** only the crate manifest carries one


### Requirement: A manual workflow run is a dry run

The system SHALL let the developer start the release workflow by hand. A
manual run SHALL build both platforms into a draft release tagged
`dry-run-<run id>` and SHALL NOT publish it.

#### Scenario: Manual run

- **WHEN** the developer starts the release workflow by hand
- **THEN** a draft release tagged `dry-run-<run id>` holds the built assets
- **AND** the release is never published

## REMOVED Requirements

### Requirement: One version source

**Reason**: Replaced by "One application version source". The "Frontend package version is inert" scenario is renamed, as there is no frontend package. OpenSpec cannot rename or drop a
scenario inside a MODIFIED block.

**Migration**: None. The rule is unchanged under the new header.
