## Purpose

Turns the commit history into versioned, downloadable builds: one local command
derives the next version and tags it, and a CI run turns that tag into a GitHub
release with installers for both platforms.

## ADDED Requirements

### Requirement: One version source

The system SHALL keep the application version in exactly one file, the Rust
crate manifest, and every produced artifact MUST report that version: the
window and installer metadata, the bundle file names, the git tag, and the
GitHub release name.

#### Scenario: Version read from the crate manifest

- **WHEN** the crate manifest says `0.3.0` and a release build runs
- **THEN** the installer metadata, the bundle file names, and the tag all carry `0.3.0`, and no other checked-in file needs an edit to agree

#### Scenario: Frontend package version is inert

- **WHEN** the version in `package.json` differs from the crate manifest
- **THEN** the build output and the release are unaffected

### Requirement: Next version derives from commit history

The system SHALL compute the next version from the conventional commit messages
since the most recent release tag: a `fix` bumps patch, a `feat` bumps minor,
and a `!` marker or `BREAKING CHANGE` footer bumps major. Commits of other types
MUST NOT bump the version on their own. The developer MUST be able to override
the derived version with an explicit level or version.

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
- **THEN** the release command produces `v0.1.0`, or the developer names the first version explicitly

#### Scenario: Manual override

- **WHEN** the developer passes an explicit version or level to the release command
- **THEN** that value is used instead of the derived one, and the rest of the release proceeds the same way

### Requirement: Release command produces one commit and one tag

The system SHALL, on one command, write the new version's section into
`CHANGELOG.md`, set the version in the crate manifest and its lockfile, create
a single commit containing exactly those changes, tag that commit `vX.Y.Z`, and
push the branch and the tag. The command MUST refuse to run on a dirty working
tree, on a branch other than `main`, or when `main` is behind its remote. A
dry run MUST be available and MUST change nothing.

#### Scenario: Clean release from main

- **WHEN** the developer runs the release command on a clean, up-to-date `main`
- **THEN** one new commit exists whose diff touches only `CHANGELOG.md`, the crate manifest, and the lockfile, it is tagged `vX.Y.Z`, and both commit and tag are on the remote

#### Scenario: Dirty working tree

- **WHEN** the developer runs the release command with uncommitted changes
- **THEN** the command aborts before writing anything and names the problem

#### Scenario: Dry run

- **WHEN** the developer runs the release command in dry-run mode
- **THEN** it prints the version it would create and the steps it would take, and the working tree, the commit history, and the tags are unchanged

#### Scenario: Release from Windows

- **WHEN** the developer runs the release command from PowerShell on the Windows machine
- **THEN** it behaves exactly as on macOS, with the pinned tool versions and without a POSIX shell on the path

#### Scenario: Release from macOS

- **WHEN** the developer runs the release command from fish on the Mac
- **THEN** it behaves exactly as on Windows

### Requirement: Changelog follows conventional commit groups

The system SHALL maintain `CHANGELOG.md` at the repository root with one
section per release, newest first, each section headed by the version and the
release date and grouped by commit type with human-readable group titles.
Commits that are not conventional, or that carry release bookkeeping, MUST NOT
appear.

#### Scenario: Section content

- **WHEN** a release contains `feat(quicklinks): show favicons` and `fix(macos): restore focus`
- **THEN** its section lists the first under a features group and the second under a fixes group, each with its scope and subject, and neither the release commit nor `docs(openspec)` commits appear

#### Scenario: Existing history

- **WHEN** the first release is cut on a repository with untagged history
- **THEN** that history forms the first section, so the changelog starts complete rather than empty

### Requirement: A tag becomes a GitHub release with installers

The system SHALL, when a `v*` tag is pushed, build release bundles for macOS on
Apple Silicon and for Windows, create a GitHub release named after the tag
whose body is that version's changelog section, attach the bundles, and publish
the release only after every platform's build succeeded. A failed build MUST
leave the release unpublished and the failure visible in the workflow run.

#### Scenario: Windows assets

- **WHEN** the tag `v0.3.0` is pushed and the workflow completes
- **THEN** the release `v0.3.0` contains a Windows `.msi` and an NSIS `.exe` installer, both installing an application that reports version `0.3.0`

#### Scenario: macOS assets

- **WHEN** the tag `v0.3.0` is pushed and the workflow completes
- **THEN** the release `v0.3.0` contains an Apple Silicon `.dmg` whose application bundle reports version `0.3.0` and launches after the one-time Gatekeeper override

#### Scenario: Release body

- **WHEN** the workflow creates the release
- **THEN** its body is the `v0.3.0` section of `CHANGELOG.md`, without the file header or other versions

#### Scenario: One platform fails

- **WHEN** the Windows build fails and the macOS build succeeds
- **THEN** the release stays a draft with only the macOS asset, and the workflow run is red

#### Scenario: Non-release pushes

- **WHEN** a commit or a tag not matching `v*` is pushed
- **THEN** the release workflow does not run

### Requirement: Release tooling is pinned per clone

The system SHALL install every tool the release command needs through the
project's mise configuration at pinned versions, so both machines produce the
same version numbers and changelog for the same history.

#### Scenario: Fresh clone

- **WHEN** the developer runs `mise install` in a fresh clone on either platform
- **THEN** the release command's dry run works without installing anything else by hand
