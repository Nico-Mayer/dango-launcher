## Context

See proposal.md for the measured costs. The facts that shape the approach:

- `mise run` installs every missing tool a task needs before it runs
  (`task.run_auto_install`, default on). For a task without its own `tools`,
  that means all of `mise.toml`. CI installs only `hk` through mise-action, so
  `mise run check` installs node, OpenSpec, git-cliff, cargo-release, and
  cargo-packager on its own. mise-action had a cache hit for the `hk` install
  and does not save again, so these tools never get cached.
- cargo-packager's GitHub releases carry no binaries. cargo-quickinstall
  publishes signed prebuilt 0.11.8 archives for `aarch64-apple-darwin` and
  `x86_64-pc-windows-msvc`. mise's cargo backend uses cargo-binstall when it
  is installed, but by default it keeps cargo-binstall away from
  quickinstall (`cargo.binstall_quickinstall`). mise's own installer
  (`cargo.binstall_native`) never uses quickinstall, so it would find nothing.
- A workflow run can restore caches from its own ref and from the default
  branch. A tag push runs on `refs/tags/vX.Y.Z`, so a cache it saves can be
  read only by that tag. Every release makes a new one.
- swatinem/rust-cache keys on the job id, and both workflows name their job
  `build`. A release restores CI's debug-profile cache, which a release build
  cannot reuse.
- rust-cache caches `./target` only. `tools/dango-ui-mcp` has its own target
  folder, which `hk check --all` builds with clippy on every run.

## Goals / Non-Goals

**Goals:**

- Each fix stands alone and can be reverted on its own.
- Local `mise install` keeps working the same way on both machines.

**Non-Goals:**

- Any change to what the workflows produce. The release-pipeline spec stays
  the contract.

## Decisions

### 1. Turn off task auto-install in CI, install tools explicitly

CI sets `MISE_TASK_RUN_AUTO_INSTALL=false` in the workflow `env` and installs
`hk` and `rust` through mise-action's `install_args`. Local runs keep
auto-install.

- Rejected: `mise run --skip-tools check`. It works, but only for that one
  call. The env var also covers any later `mise run` step.
- Rejected: giving the `check` task its own `tools`. It changes local
  behaviour to fix a CI problem.
- Rejected: `install: true` without args, so mise-action installs and caches
  all tools. CI would still pay for tools it never uses on each cache miss.

### 2. cargo-packager through cargo-binstall and quickinstall

`mise.toml` adds `cargo-binstall` at a pinned version and sets
`cargo.binstall_quickinstall = true` in `[settings]`. The release workflow
installs `cargo-binstall` with `cargo:cargo-packager`. The pinned
`cargo:cargo-packager` version stays the single source.

- Rejected: `cargo.binstall_native`. It reads only the crate's own release
  artifacts, and cargo-packager publishes none.
- Rejected: a `github:` or `ubi:` backend on cargo-packager's repo. There are
  no assets to download.
- Rejected: caching the source-built binary with `actions/cache`. Tags cannot
  share caches with each other, so it would need a cache built on `main` just
  for this.
- Rejected: taiki-e/install-action. It works, but it puts a second version
  pin outside `mise.toml`, and local installs would still compile.

### 3. No Rust build cache in the release workflow

Remove `swatinem/rust-cache` from `release.yml`.

- Rejected: `save-if: false`. It stops the useless save, but the restore still
  downloads CI's whole debug cache (about 1 GB) for nothing usable.
- Rejected: `cache-targets: false`. The registry is a small part of the
  release time, and the cache key would still collide with CI's.
- Rejected: building a release-profile cache on `main` for tags to read. It
  adds a full LTO build to every push to save time on rare releases.

### 4. Cache the dango-ui-mcp build in CI

Set rust-cache `workspaces` to `.` and `tools/dango-ui-mcp`, each with its own
`target`.

- Rejected: leaving the MCP proxy out of `hk check --all` in CI. CI should
  lint the same files as the local check.

### 5. Cancel superseded CI runs

`concurrency: { group: ${{ github.workflow }}-${{ github.ref }},
cancel-in-progress: true }` in `ci.yml`, for pushes to `main` and for pull
requests. The newest commit's result covers the older ones. `release.yml`
gets no concurrency group: a release must never be cancelled.

### 6. Skip CI for docs-only pushes

`paths-ignore` on both triggers: `openspec/**`, `docs/**`, `**/*.md`. A
release commit still runs CI, because it touches `Cargo.toml` and
`Cargo.lock`.

- Rejected: a job-level path filter action. There is no required status
  check to keep green, so the built-in filter is enough.

### 7. Dev Drive trial on Windows

Try `samypr100/setup-dev-drive` in the Windows CI job, with the Cargo home
and the target folder on the drive. Keep it only if the Check and Test steps
get clearly faster over a few runs. Otherwise remove it.

## Risks / Trade-offs

- [The release job, which holds the signing secrets, runs a third-party
  binary from quickinstall] -> cargo-binstall checks the signature of
  quickinstall archives. The binary is the same crate at the same pinned
  version. If this is not acceptable, revert decision 2 alone and keep the
  source build.
- [cargo-binstall finds no prebuilt binary after a version bump] -> mise falls
  back to `cargo install`, so the build is only slow, not broken.
- [mise installs cargo-packager before cargo-binstall] -> check the install
  log of the first run. If it built from source, list `cargo-binstall` in a
  separate earlier install.
- [Auto-install off means a new tool that a CI task needs fails CI] -> the
  error names the missing tool. Add it to `install_args`. A new AGENTS.md
  build fact records why.
- [Cancelling a `main` run also cancels its cache save] -> the next run saves
  it. The cost is one colder run at most.
- [A docs-only push leaves no CI result] -> acceptable: those files do not
  reach the build.

## Migration Plan

Each decision is its own commit. After the release workflow change lands,
delete the old caches under `refs/tags/*` with `gh cache delete`. To roll
back, revert the commit for that decision.
