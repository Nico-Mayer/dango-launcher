## 1. CI installs only what the check needs

- [ ] 1.1 In `ci.yml`, set `MISE_TASK_RUN_AUTO_INSTALL: "false"` in the workflow `env` and set mise-action `install_args` to `hk rust`. Verify: in the CI run's Check step log, mise installs no tools, and both jobs pass
- [ ] 1.2 Add rust-cache `workspaces` for `.` and `tools/dango-ui-mcp`, each with its own `target`, in `ci.yml`. Verify: the second CI run after the change shows no `Compiling` lines for the proxy's dependencies in the Check step

## 2. Prebuilt cargo-packager

- [ ] 2.1 Add `cargo-binstall` pinned to `1.26.2` to `mise.toml` `[tools]` and `cargo.binstall_quickinstall = true` to `[settings]`. Verify: after `mise uninstall cargo:cargo-packager`, `mise install` on the Mac downloads cargo-packager without compiling, and `cargo packager --version` prints `0.11.8`
- [ ] 2.2 Set the release workflow's mise-action `install_args` to `cargo-binstall cargo:cargo-packager`, dropping the redundant `rust`. Verify: a `workflow_dispatch` dry run shows cargo-packager installed by cargo-binstall in seconds on both jobs, and both packages upload to the draft release

## 3. Release workflow without a Rust build cache

- [ ] 3.1 Remove `swatinem/rust-cache` from `release.yml`. Verify: a `workflow_dispatch` dry run has no cache restore or save steps, both jobs upload their assets, and `gh cache list` shows no new `refs/tags` or dry-run entry
- [ ] 3.2 Delete the existing caches under `refs/tags/*` with `gh cache delete`, then delete the dry-run draft releases from 2.2 and 3.1. Verify: `gh cache list` shows only `refs/heads/main` entries and the total stays under 10 GB

## 4. Fewer CI runs

- [ ] 4.1 Add a `concurrency` group on workflow and ref with `cancel-in-progress: true` to `ci.yml`. Verify: two pushes in a row show the first run as cancelled in `gh run list`
- [ ] 4.2 Add `paths-ignore` for `openspec/**`, `docs/**`, and `**/*.md` to both CI triggers. Verify: a push that touches only `openspec/` starts no CI run in `gh run list`, and the next push that touches `src/` starts one

## 5. Dev Drive trial on Windows

- [ ] 5.1 Add `samypr100/setup-dev-drive` to the Windows CI job with the Cargo home and the target folder on the drive. Compare the Check and Test step times over three runs with the runs from group 1 (`gh run view --json jobs`). Keep it if both are clearly faster, otherwise revert the commit, and record the numbers in this task either way

## 6. Docs and review

- [ ] 6.1 Add to AGENTS.md "Build and verification facts": CI turns off mise task auto-install because `mise run` would install every tool in `mise.toml`, and a cache saved by a tag run can only be read by that tag. Verify: both facts are in AGENTS.md, and no other file repeats them
- [ ] 6.2 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context. The review reports findings only; the author decides what to fix
