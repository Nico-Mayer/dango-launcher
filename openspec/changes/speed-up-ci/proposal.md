## Why

CI and releases spend much of their time on caching mistakes, not on the
build. Measured on the runs of 2026-11-01:

- A Windows CI run takes about 5.5 min. About 2 min of it is `mise run check`
  installing every tool in `mise.toml`, with cargo-packager compiled from
  source, on every run.
- A release takes about 17 min. Each platform job spends 2.5-3 min compiling
  cargo-packager, then about 1.5-2 min restoring a debug cache it cannot use
  and saving a cache under the tag that no later run can read.
- The repository's caches total about 10.4 GB, over the 10 GB limit, so
  GitHub evicts entries. The unreadable tag caches cause most of it.

## What Changes

- CI installs only the tools the check needs, and `mise run` no longer
  installs the rest of `mise.toml` on its own.
- cargo-packager installs as a prebuilt binary through cargo-binstall,
  locally and in the release workflow, instead of compiling from source.
- The release workflow stops using the Rust build cache: it neither restores
  CI's debug cache nor saves caches under tags.
- CI caches the `tools/dango-ui-mcp` build as well, so its dependencies stop
  compiling from scratch on every run.
- A new push cancels the older CI run on the same branch or pull request.
- CI skips pushes that touch only `openspec/`, `docs/`, or Markdown files.
- A trial of a Dev Drive for the Windows CI job, kept only if it makes the
  job faster.

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

## Platforms

Both CI jobs (macOS and Windows) and both release jobs are covered. The Dev
Drive trial applies to the Windows CI job only, because Dev Drive is a
Windows file system feature.

## Impact

- `.github/workflows/ci.yml` and `.github/workflows/release.yml`.
- `mise.toml`: adds `cargo-binstall` as a pinned tool and allows cargo-binstall
  to use the cargo-quickinstall artifact host. Local `mise install` on both
  machines then fetches cargo-packager prebuilt too.
- `AGENTS.md`: new build facts on mise auto-install and on tag-scoped caches.
- GitHub Actions cache: old tag caches can be deleted once the release
  workflow stops writing them.
