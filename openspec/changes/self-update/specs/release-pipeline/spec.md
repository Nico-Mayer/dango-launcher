## MODIFIED Requirements

### Requirement: A tag becomes a GitHub release with installers

The system SHALL, when a `v*` tag is pushed, build release bundles for macOS on
Apple Silicon and for Windows, create a GitHub release named after the tag
whose body is that version's changelog section, attach the bundles, and publish
the release only after every platform's build succeeded. A failed build MUST
leave the release unpublished and the failure visible in the workflow run.

Every release SHALL also carry the updater artifacts for both platforms, each
signed with the project's updater key, and one update manifest naming the
version, the artifact for each platform, and its signature. On Windows the
manifest SHALL point at the NSIS installer. The updater key and the macOS
signing certificate SHALL come from repository secrets and MUST NOT be checked
in.

The macOS application SHALL be signed with the same code signing identity in
every release, so its designated requirement stays the same from one version
to the next.

#### Scenario: Windows assets

- **WHEN** the tag `v0.3.0` is pushed and the workflow completes
- **THEN** the release `v0.3.0` contains a Windows `.msi` and an NSIS `.exe` installer, both installing an application that reports version `0.3.0`

#### Scenario: macOS assets

- **WHEN** the tag `v0.3.0` is pushed and the workflow completes
- **THEN** the release `v0.3.0` contains an Apple Silicon `.dmg` whose application bundle reports version `0.3.0` and launches after the one-time Gatekeeper override

#### Scenario: Update manifest

- **WHEN** the tag `v0.3.0` is pushed and the workflow completes
- **THEN** the release `v0.3.0` contains a `latest.json` naming version `0.3.0`, a signed Windows NSIS artifact, and a signed macOS Apple Silicon artifact
- **AND** each artifact's signature verifies against the public key built into Dango

#### Scenario: Latest manifest URL

- **WHEN** `v0.3.0` is the newest published release
- **THEN** the repository's latest-release download URL for `latest.json` returns the `v0.3.0` manifest

#### Scenario: Stable macOS signing identity

- **WHEN** the macOS application bundles of two consecutive releases are compared
- **THEN** both are signed with the same identity, with the bundle identifier as the signing identifier
- **AND** both report the same designated requirement

#### Scenario: Permissions survive an update on macOS

- **WHEN** Accessibility and Input Monitoring are granted to an installed release and it is updated to the next release
- **THEN** both permissions are still granted to the updated Dango without the user granting them again

#### Scenario: Release body

- **WHEN** the workflow creates the release
- **THEN** its body is the `v0.3.0` section of `CHANGELOG.md`, without the file header or other versions

#### Scenario: One platform fails

- **WHEN** the Windows build fails and the macOS build succeeds
- **THEN** the release stays a draft with only the macOS asset, and the workflow run is red

#### Scenario: Non-release pushes

- **WHEN** a commit or a tag not matching `v*` is pushed
- **THEN** the release workflow does not run
