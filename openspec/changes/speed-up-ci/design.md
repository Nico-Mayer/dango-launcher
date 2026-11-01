## Context

See proposal.md for the measured costs. The facts that shape the approach:

- `mise run` installs every missing tool a task needs before it runs
  (`task.run_auto_install`, default on). For a task without its own `tools`,
  that means all of `mise.toml`. CI installs only `hk` through mise-action, so
  `mise run check` installs node, OpenSpec, git-cliff, cargo-release, and
  cargo-packager on its own. mise-action had a cache hit for the `hk` install
  and does not save again, so these tools never get cached.
- A workflow run can restore caches from its own ref and from the default
  branch. A tag push runs on `refs/tags/vX.Y.Z`, so a cache it saves can be
  read only by that tag. Every release makes a new one.
- swatinem/rust-cache keys on the job id, and both workflows name their job
  `build`. A release restores CI's debug-profile cache, which a release build
  cannot reuse.

## Goals / Non-Goals

**Goals:**

- Each fix stands alone and can be reverted on its own.

**Non-Goals:**

- Any change to what the workflows produce. The release-pipeline spec stays
  the contract.

## Decisions

### 1. Turn off task auto-install in CI

CI sets `MISE_TASK_RUN_AUTO_INSTALL=false` in the workflow `env`. mise-action
keeps `install_args: hk`; the "Install Rust" step already installs the
toolchain. Local runs keep auto-install.

- Rejected: `mise run --skip-tools check`. It works, but only for that one
  call. The env var also covers any later `mise run` step.
- Rejected: giving the `check` task its own `tools`. It changes local
  behaviour to fix a CI problem.
- Rejected: `install: true` without args, so mise-action installs and caches
  all tools. CI would still pay for tools it never uses on each cache miss.

### 2. No Rust build cache in the release workflow

Remove `swatinem/rust-cache` from `release.yml`, and set `cache_save: false`
on its mise-action step, whose tool cache would also land under the tag.

- Rejected: `save-if: false`. It stops the useless save, but the restore still
  downloads CI's whole debug cache (about 1 GB) for nothing usable.
- Rejected: `cache-targets: false`. The registry is a small part of the
  release time, and the cache key would still collide with CI's.
- Rejected: building a release-profile cache on `main` for tags to read. It
  adds a full LTO build to every push to save time on rare releases.

### 3. Cancel superseded CI runs

`concurrency: { group: ${{ github.workflow }}-${{ github.ref }},
cancel-in-progress: true }` in `ci.yml`, for pushes to `main` and for pull
requests. The newest commit's result covers the older ones. `release.yml`
gets no concurrency group: a release must never be cancelled.

### 4. Skip CI for docs-only pushes

`paths-ignore` on both triggers: `openspec/**`, `docs/**`, `**/*.md`. A
release commit still runs CI, because it touches `Cargo.toml` and
`Cargo.lock`.

- Rejected: a job-level path filter action. There is no required status
  check to keep green, so the built-in filter is enough.

## Risks / Trade-offs

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
