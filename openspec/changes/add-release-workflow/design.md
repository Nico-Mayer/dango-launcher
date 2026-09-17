## Context

See proposal.md for motivation. What shapes the approach:

- The version appears in three checked-in files today, all `0.1.0`:
  `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, and `package.json`.
  `src-tauri/Cargo.lock` is committed and carries the crate version too. The
  Tauri v2 config reference states that when `version` is absent from
  `tauri.conf.json`, "the version number from Cargo.toml is used".
- The repository has no tags. History is conventional commits throughout,
  including many `docs(openspec)` commits that should not appear in a changelog.
- `ci.yml` already builds on `macos-latest` and `windows-latest` with
  `dtolnay/rust-toolchain`, `swatinem/rust-cache`, and `actions/setup-node`.
  `macos-latest` is an Apple Silicon runner. `bundle.targets` is `all`, which on
  Windows yields both a WiX `.msi` and an NSIS `.exe`, and on macOS a `.dmg`
  and `.app`.
- Tooling is pinned in `mise.toml`; the hk change set the precedent for adding
  a developer tool there and documenting one setup command. mise runs TOML
  tasks through `cmd /c` on Windows and `sh -c` elsewhere, so a task body
  cannot rely on POSIX syntax. mise renders Tera templates in task bodies, and
  `exec(command=...)` returns a command's output.
- The pre-push hook from `local-checks` runs clippy and `svelte-check` on every
  push, including the one the release task makes.
- Verified upstream state at the time of writing: git-cliff 2.14.1
  (2026-09-01, `aqua:orhun/git-cliff` in the mise registry), cargo-release
  (`github:crate-ci/cargo-release`), tauri-action 1.0.0 (drops Tauri v1, needs
  runner 2.327.1 or newer, renames `includeUpdaterJson` to `uploadUpdaterJson`),
  git-cliff-action v4.

## Goals / Non-Goals

**Goals:**

- The whole local half is one mise task that works identically from PowerShell
  and fish, with a dry run as the default safety net.
- The CI half is one workflow file that a reader can follow top to bottom.
- Every tool is off the shelf and pinned. Nothing in the repo parses commits,
  edits manifests, or talks to the GitHub API by hand.

**Non-Goals:**

- Anything the proposal lists. In particular the release workflow does not
  become a second CI: it builds, it does not lint or test.

## Decisions

### Cargo.toml is the single version source

Alternatives:

- **`tauri.conf.json` pointing at `../package.json`**, Tauri's documented
  option. Rejected: it moves the truth to the frontend package for an app whose
  logic and identity are Rust, and Cargo.toml would still need to agree for
  `cargo` metadata. Two files again.
- **Keep all three and let the release task rewrite them.** Rejected: three
  hand-written edits, three chances to drift, and a bespoke script where a
  documented fallback exists.

Chosen: delete `version` from `tauri.conf.json` and let Tauri read Cargo.toml.
`package.json` keeps `"version": "0.1.0"` as an inert placeholder; a comment is
not possible in JSON, so the README states that the field is not the app
version. Verification: `npx tauri build --no-bundle` after the deletion must
still produce a binary that reports the crate version.

Before the first release the crate manifest says `0.0.0`, not `0.1.0`.
cargo-release accepts a version equal to the current one, but then it has no
manifest change to commit, so the hook's changelog would be left uncommitted
and the tag would land on the previous commit. Starting from `0.0.0` makes
`v0.1.0` a real bump with one commit like every later release.

### git-cliff computes the version and writes the changelog

This is the svu role. Candidates checked: git-cliff, cocogitto, convco,
release-plz, semantic-release, release-please.

- **cocogitto** (`cog bump --auto`): does version, changelog, commit, and tag
  in one command with hooks. It still needs a separate tool in a
  `pre_bump_hook` to set the Cargo version, so it does not remove a dependency,
  and its changelog templates are less flexible. Viable; rejected because the
  git-cliff plus cargo-release split gives each tool one job.
- **convco**: `convco version --bump` and `convco changelog`. Smaller
  community, no `initial_tag`, and no GitHub Action to reuse in CI. Rejected.
- **release-plz**: a release-PR bot built around crates.io publishing with
  `cargo-semver-checks`. Far more machinery than a hobby desktop app wants, and
  it does not do the "one command from my terminal" flow. Rejected.
- **semantic-release / release-please**: Node bots, JavaScript plugin
  configuration, and a PR-based flow. Rejected for the same reason plus a
  second toolchain for release logic.

Chosen: git-cliff. `git cliff --bumped-version` prints the next version from
the commits since the last tag. `git cliff --tag vX.Y.Z --output CHANGELOG.md`
regenerates the whole file from history with the unreleased commits under the
new version. Regenerating replaced the planned `--prepend`: prepend fails when
the file does not exist, and a regenerated file is byte-identical when the
history is, which keeps a "nothing to release" run from dirtying the tree. A
header-only `CHANGELOG.md` is committed with the tooling because cargo-release
commits with `git commit -a`, which ignores untracked files. The same binary
runs in CI through
`orhun/git-cliff-action@v4` with `--latest --strip header` to produce the
release body, so the release notes and `CHANGELOG.md` cannot disagree.

`cliff.toml` settings that matter: `tag_pattern = "v[0-9].*"`; `[bump]
initial_tag = "v0.1.0"` so the first run on an untagged repo produces the
version Cargo.toml already has; `commit_parsers` that drop `chore(release)`
and `docs(openspec)` and map the remaining types to group titles;
`filter_unconventional = true`. Group titles in the changelog are project copy
and are quoted below.

### cargo-release performs the bump, commit, tag, and push

This is the part goreleaser leaves to the developer. Candidates checked for
setting the version in Cargo.toml and Cargo.lock: cargo-edit
(`cargo set-version`), cargo-release, a `sed` line plus `cargo update
--workspace`, cargo-bump.

- **cargo-edit**: `cargo set-version X` is the obvious fit, but it is not in the
  mise registry under any name, its documentation does not say whether it
  updates `Cargo.lock`, and installing through the `cargo:` backend compiles it
  from source on each machine. Rejected.
- **`sed` plus `cargo update --workspace`**: two lines, but hand-rolled, and
  `sed` behaves differently on macOS and is absent from `cmd`. Rejected under
  the off-the-shelf rule.
- **cargo-bump**: unmaintained and only edits the manifest. Rejected.
- **cargo-release**: accepts an explicit version (`cargo release 0.3.0`), sets
  the manifest and lockfile, runs a `pre-release-hook` with `{{version}}`
  available, makes one commit, tags with a configurable `tag-name`, pushes, and
  refuses to run on a dirty tree, a disallowed branch, or a branch behind its
  remote. Dry run is the default and `--execute` is required to act, which is
  the safety property the spec asks for. It is in the mise registry as
  `github:crate-ci/cargo-release`. Chosen.

cargo-release does not compute the version from commits. That is exactly what
git-cliff supplies, so the task composes them:

```toml
[tools]
"aqua:orhun/git-cliff" = "2.14.1"
"github:crate-ci/cargo-release" = "1.1.6"

[tasks.release]
description = "Cut a release: derive version, changelog, commit, tag, push"
run = "node scripts/release.mjs"
```

The planned one-liner with `{{exec(command='git cliff --bumped-version')}}`
failed: mise renders the template before the task's tools are on `PATH`, so
`git cliff` is not found. The fallback from the risks section is in place:
`scripts/release.mjs` runs `git cliff --bumped-version` from the repo root,
strips the `v`, and runs `cargo release <version>` in `src-tauri` with any
extra arguments appended. mise appends everything after `--` to the command,
so `mise run release -- --execute` needs no argument declaration. With no
flags the task is a dry run that prints the derived version and every step.
An explicit level or version replaces the derived one by running `cargo
release minor --execute` directly; that is the manual override and does not
need a second task.

`src-tauri/release.toml` as validated against cargo-release 1.1.6:

```toml
publish = false
allow-branch = ["main"]
tag-name = "v{{version}}"
pre-release-commit-message = "chore(release): v{{version}}"
pre-release-hook = ["node", "../scripts/changelog.mjs"]
```

The hook is a script rather than the git-cliff command itself for two
reasons found in the dry run. cargo-release runs the hook in dry-run mode too
and only tells it through the `DRY_RUN` environment variable, so a plain
`git cliff --output` would write the changelog during a rehearsal. And the
hook's working directory is the crate directory, from which git-cliff limits
itself to commits touching `src-tauri`. `scripts/changelog.mjs` runs git-cliff
from the repo root with `--tag v$NEW_VERSION`, printing the unreleased section
when `DRY_RUN` is `true` and writing `CHANGELOG.md` otherwise. Neither script
parses commits or edits manifests; both only choose arguments for the tools.

Cargo.lock: confirmed in the dry run and a local `--execute --no-push` run.
cargo-release refreshes the crate's own entry, and the release commit touches
exactly `CHANGELOG.md`, `Cargo.toml`, and `Cargo.lock`.

### Ignore the "nothing to release" case rather than script it

When no bumping commit exists, `git cliff --bumped-version` prints the current
version. cargo-release accepts an equal version but refuses because the tag
exists: "tag `v0.1.0` already exists (for `dango`)", exit code 101, nothing
written, in dry-run and execute mode alike. That satisfies the spec's scenario
without any logic in the task; the message names the existing tag rather than
saying "nothing to release" literally, which is accepted.

### tauri-action builds and uploads; a separate job owns the release

This is the goreleaser role. Candidates checked: tauri-action, cargo-dist,
hand-written `tauri build` plus `softprops/action-gh-release`.

- **cargo-dist**: runs `cargo build` and packages the binary. It never invokes
  `tauri build`, so no `.dmg`, `.msi`, or NSIS installer. Wrong tool.
- **Hand-written build plus upload action**: `npm run tauri build`, then glob
  the bundle directory per platform and upload. Works, but duplicates path
  knowledge tauri-action already has and needs per-OS glob patterns.
- **tauri-action**: official, installs nothing extra, knows the bundle paths,
  and uploads to a release by tag or by id. Chosen.

Release creation is pulled out of the build matrix into its own job. The
official example lets each matrix job call tauri-action with `tagName` and
`releaseName`, which makes two jobs race to create the same release and leaves
the release as a draft to publish by hand. Instead:

```
 on: push tags v*
 +----------------+     +---------------------------+     +----------------+
 | changelog      | --> | build (matrix)            | --> | publish        |
 | git-cliff-     |     | macos-latest, windows-    |     | gh release     |
 | action, then   |     | latest; tauri-action with |     | edit --draft=  |
 | gh release     |     | releaseId, uploadUpdater- |     | false          |
 | create --draft |     | Json false                |     |                |
 +----------------+     +---------------------------+     +----------------+
```

- `changelog` checks out with full history (`fetch-depth: 0`, git-cliff needs
  tags), runs `orhun/git-cliff-action@v4` with `--latest --strip header`,
  creates a draft release with `gh release create "$TAG" --draft --title
  "Dango $TAG" --notes-file`, and outputs the numeric release id from `gh
  release view --json databaseId`.
- `build` reuses the `ci.yml` setup steps (node, rust toolchain, rust cache,
  `npm ci`) and calls `tauri-apps/tauri-action@v1` with `releaseId`,
  `uploadUpdaterJson: false`, and no `args`, since `macos-latest` is already
  Apple Silicon. `fail-fast: false` so one platform's failure does not cancel
  the other's upload.
- `publish` runs with `needs: build` and flips the draft to published. If any
  build failed, `publish` is skipped and the draft stays visible with whatever
  assets made it, which is the spec's "one platform fails" scenario.

`permissions: contents: write` on the workflow. `gh` is preinstalled on GitHub
runners and authenticates from `GITHUB_TOKEN`.

### Release workflow does not repeat CI

`ci.yml` runs on every push to `main`. The tag points at a commit on `main`, so
the checks already ran, and the pre-push hook ran them again locally before
the tag left the machine. Repeating clippy and tests would double the macOS
minutes for no new information.

### Changelog copy

The changelog is text the user reads, so its group titles follow the
interface-copy spec: sentence case, no internal identifiers. Titles by commit
type:

| type | title |
|---|---|
| `feat` | Features |
| `fix` | Fixes |
| `perf` | Performance |
| `refactor` | Refactoring |
| `docs` (not `docs(openspec)`) | Documentation |
| `test`, `ci`, `build`, `chore`, `style` | Maintenance |

Skipped entirely: `chore(release)`, `docs(openspec)`, unconventional subjects.
The file header reads:

> # Changelog
>
> All notable changes to Dango. Generated from commit history at release time.

## Risks / Trade-offs

- [`{{exec}}` in a mise task fails] → Happened on macOS already: the template
  renders before tools are on `PATH`. Resolved with `scripts/release.mjs`,
  run through `node` from a TOML task so Windows needs no shebang handling.
- [cargo-release does not refresh `Cargo.lock`] → Did not happen; confirmed
  refreshed in a local execute run.
- [`git cliff --bump` and `--tag` conflict in the hook] → Moot; the hook passes
  only `--tag` with the version cargo-release supplies.
- [Two-job release creation is more YAML than the official example] → Accepted
  for deterministic release creation and automatic publish. The extra job is
  about fifteen lines.
- [Unsigned macOS app blocked by Gatekeeper] → Documented one-time `xattr -cr
  Dango.app` in the README. Signing stays deferred per M7.
- [Release build fails only on the tag because `ci.yml` never bundles] →
  `ci.yml` builds the frontend and the crate but not the installers, so WiX or
  DMG packaging problems surface only at release time. Accepted; the fix is a
  new tag after the fix commit, and no user is waiting.
- [Pre-push hook slows the release push] → It is the same clippy and
  `svelte-check` the developer just ran to land the last commit, so incremental
  and fast. Accepted.

## Migration Plan

1. Land the tooling and workflow on `main` without a tag. CI stays green
   because nothing in `ci.yml` changes.
2. Run `mise run release` (dry run) on both machines and check the derived
   version is `0.1.0` from `initial_tag`, and the hook output looks right.
   Done on macOS from fish; the Windows run is still open.
3. Cut `v0.1.0` with `--execute` from one machine. Watch the workflow, install
   both bundles, confirm the version in the installer metadata.
4. Rollback: delete the tag locally and on the remote and revert the release
   commit. Nothing outside git and the GitHub release holds state.

## Open Questions

- Whether cargo-release 1.1.6's Windows asset
  (`cargo-release-v1.1.6-x86_64-pc-windows-msvc.zip`) resolves through the
  `github:` backend without an `asset` pattern. The macOS asset did; the
  Windows `mise install` in task 2.6 settles it.
