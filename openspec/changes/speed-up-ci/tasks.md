## 1. CI installs only what the check needs

- [ ] 1.1 In `ci.yml`, set `MISE_TASK_RUN_AUTO_INSTALL: "false"` in the workflow `env`, keeping mise-action `install_args: hk`. Verify: in the CI run's Check step log, mise installs no tools, and both jobs pass

## 2. Release workflow without a Rust build cache

- [ ] 2.1 Remove `swatinem/rust-cache` from `release.yml` and set `cache_save: false` on its mise-action step. Verify: a `workflow_dispatch` dry run has no cache restore or save steps, both jobs upload their assets, and `gh cache list` shows no new `refs/tags` or dry-run entry
- [ ] 2.2 Delete the existing caches under `refs/tags/*` with `gh cache delete`, then delete the dry-run draft release from 2.1. Verify: `gh cache list` shows only `refs/heads/main` entries and the total stays under 10 GB

## 3. Fewer CI runs

- [ ] 3.1 Add a `concurrency` group on workflow and ref with `cancel-in-progress: true` to `ci.yml`. Verify: two pushes in a row show the first run as cancelled in `gh run list`
- [ ] 3.2 Add `paths-ignore` for `openspec/**`, `docs/**`, and `**/*.md` to both CI triggers. Verify: a push that touches only `openspec/` starts no CI run in `gh run list`, and the next push that touches `src/` starts one

## 4. Docs and review

- [x] 4.1 Add to AGENTS.md "Build and verification facts": CI turns off mise task auto-install because `mise run` would install every tool in `mise.toml`, and a cache saved by a tag run can only be read by that tag. Verify: both facts are in AGENTS.md, and no other file repeats them
- [x] 4.2 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context. The review reports findings only; the author decides what to fix
