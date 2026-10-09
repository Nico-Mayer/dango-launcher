## Why

Dango has no way to ship a build. There are no tags, no releases, and the only
binaries are whatever the author last built on each machine. Installing the
launcher on the Windows machine after a change made on the Mac means pushing,
pulling, and building by hand. One command that cuts a versioned release, and a
GitHub Actions run that turns it into downloadable installers, closes that gap
with the same shape as svu plus goreleaser in the Go world.

## What changes

- Add a `release` mise task. It derives the next version from the conventional
  commits since the last tag, writes the new section into `CHANGELOG.md`, bumps
  the crate version, commits, tags `vX.Y.Z`, and pushes. No manual version
  argument by default.
- Add a `release.yml` GitHub Actions workflow triggered by a `v*` tag push. It
  builds the Tauri bundles on macOS (Apple Silicon) and Windows, creates the
  GitHub release with the changelog section as its body, uploads the installers,
  and publishes the release only after every build succeeded.
- Make `src-tauri/Cargo.toml` the single version source. **BREAKING** for
  tooling only: the `version` field leaves `tauri.conf.json`, so Tauri falls
  back to the crate version, and `package.json` stays at a frozen placeholder.
- Pin the release tooling (git-cliff, cargo-release) in `mise.toml` so both
  machines run the same versions, following the hk precedent.
- Add a `CHANGELOG.md` at the repo root, generated from the existing history at
  first release and appended by every release after that.
- Document the release procedure in `README.md`.

## Capabilities

### New capabilities

- `release-pipeline`: versioning from conventional commits, changelog
  generation, tag creation, and the CI build that turns a tag into a GitHub
  release with installers for both platforms.

### Modified capabilities

None. `local-checks` stays as it is; the release task runs through the same
pre-push hook as any other push.

## Impact

- New files: `.github/workflows/release.yml`, `cliff.toml`, `CHANGELOG.md`,
  `src-tauri/release.toml`, `scripts/release.mjs`, `scripts/changelog.mjs`.
- Edited files: `mise.toml` (tools and task), `src-tauri/tauri.conf.json`
  (drop `version`), `README.md` (release procedure). `src-tauri/Cargo.toml` and
  `Cargo.lock` change on every release, by the tool, never by hand.
- Dependencies: git-cliff and cargo-release as mise-managed developer tools;
  `tauri-apps/tauri-action@v1` and `orhun/git-cliff-action@v4` in CI. No
  runtime dependency of the app changes.
- CI: the existing `ci.yml` is untouched. Release builds run in a separate
  workflow and do not repeat clippy or tests; a tag is expected to point at a
  commit that already passed CI on `main`.
- Platforms: Windows (`.msi` and NSIS `.exe`) and macOS on Apple Silicon
  (`.dmg`). Intel macOS is not built: the author's Mac is an M2 Pro and no Intel
  machine exists to install on. The matrix entry is one line to add later.
  Linux is out of scope as everywhere in this project.
- Milestone: M7 - polish in `openspec/ROADMAP.md`. Code signing and
  auto-update stay deferred as that milestone states; this change only makes
  builds downloadable.

## Non-goals

- Code signing or notarisation on either platform. The macOS `.dmg` is unsigned
  and needs a one-time `xattr -cr` after download; the Windows installer shows
  a SmartScreen warning. Both are accepted for a single user.
- The Tauri updater, `latest.json`, or any in-app update check.
- Intel macOS or universal macOS binaries.
- Publishing to crates.io, Homebrew, winget, Scoop, or any store.
- Commit message linting or a commit hook that enforces conventional commits.
- Pre-release channels, nightlies, or builds on every push to `main`.
- Changing what `ci.yml` checks, or making the release workflow re-run those
  checks.
