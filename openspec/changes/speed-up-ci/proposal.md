## Why

CI and releases spend much of their time on caching mistakes, not on the
build. Measured on the runs of 2026-11-01:

- A Windows CI run takes about 5.5 min. About 2 min of it is `mise run check`
  installing every tool in `mise.toml`, with cargo-packager compiled from
  source, on every run.
- A release takes about 17 min. Each platform job spends about 1.5-2 min
  restoring a debug cache it cannot use and saving a cache under the tag that
  no later run can read.
- The repository's caches total about 10.4 GB, over the 10 GB limit, so
  GitHub evicts entries. The unreadable tag caches cause most of it.

## What Changes

- CI installs only the tools the check needs, and `mise run` no longer
  installs the rest of `mise.toml` on its own.
- The release workflow stops using the Rust build cache and saves no caches
  under tags: it neither restores CI's debug cache nor writes caches no later
  run can read.
- A new push cancels the older CI run on the same branch or pull request.
- CI skips pushes that touch only `openspec/`, `docs/`, or Markdown files.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. The `release-pipeline` requirements stay as they are: same triggers,
same assets, same signing, same publish rule. Only how fast the workflows get
there changes, so the change sets `skip_specs: true`.

## Non-goals

- Thin LTO, fewer codegen units, or any other release profile change. Left for
  a later decision.
- sccache, larger or paid runners, cargo-nextest, or splitting clippy and
  tests into separate jobs. Compute is free for this public repo, and these
  add moving parts for little gain.
- Changing what CI checks or what a release contains.
- Caching release builds across tags.
- A prebuilt cargo-packager in the release workflow. It saves the most per
  release, but it changes `mise.toml` and runs a third-party binary in the job
  that holds the signing secrets. It gets its own change after a local check.
- Caching the `tools/dango-ui-mcp` build. hk runs its clippy in parallel with
  the main crate's, which takes longer, so the cache would save almost nothing.
- A Dev Drive on the Windows CI job. It is an experiment, not a known win.

## Platforms

Both CI jobs (macOS and Windows) and both release jobs are covered.

## Impact

- `.github/workflows/ci.yml` and `.github/workflows/release.yml`.
- `AGENTS.md`: new build facts on mise auto-install and on tag-scoped caches.
- GitHub Actions cache: old tag caches can be deleted once the release
  workflow stops writing them.
