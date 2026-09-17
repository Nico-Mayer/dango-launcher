## 1. Single version source

- [x] 1.1 Remove `version` from `src-tauri/tauri.conf.json`; verify `npx tauri build --no-bundle` succeeds and the resulting binary's file metadata (Windows: file properties; macOS: `Info.plist` `CFBundleShortVersionString`) reports `0.1.0` from Cargo.toml
- [x] 1.2 Add a README note that `package.json`'s `version` is not the app version and that Cargo.toml is; verify `npm run build` still succeeds with the placeholder left in place

## 2. Local release tooling

- [x] 2.1 Pin `aqua:orhun/git-cliff` at 2.14.1 and `github:crate-ci/cargo-release` at its current release in `mise.toml`; verify `mise install` completes on the current machine and `git cliff --version` and `cargo release --version` print the pinned versions
- [x] 2.2 Add `cliff.toml` with `tag_pattern`, `[bump] initial_tag = "v0.1.0"`, `filter_unconventional`, and commit parsers implementing the group titles and skips from design.md; verify `git cliff --bumped-version` prints `v0.1.0` on the current untagged history and `git cliff --unreleased` output shows no `docs(openspec)` or unconventional entries and uses the quoted group titles
- [x] 2.3 Add `src-tauri/release.toml` with `publish = false`, `tag-name`, `pre-release-commit-message`, `allow-branch`, and the git-cliff `pre-release-hook`; verify `cargo release 0.1.0` from `src-tauri` in dry-run mode lists the hook, the commit `chore(release): v0.1.0`, the tag `v0.1.0`, and the push, and leaves the tree clean
- [x] 2.4 Add the `release` mise task from design.md; verify `mise run release` prints the same dry run as 2.3 with the version filled in by `{{exec}}`, and `mise run release -- --execute` is accepted as the flags argument (do not execute yet)
- [x] 2.5 Verify the refusal paths in dry-run and execute mode without releasing: a dirty tree, a non-`main` branch, and a version equal to the current one each exit non-zero with nothing written
- [ ] 2.6 Verify the release task on Windows from PowerShell: `mise install`, then `mise run release` dry run prints the derived version and steps without a POSIX shell involved
- [x] 2.7 Verify the release task on macOS from fish: `mise install`, then `mise run release` dry run prints the identical version and steps

## 3. Release workflow

- [x] 3.1 Add `.github/workflows/release.yml` triggered on `push` of `v*` tags with `permissions: contents: write`, a `changelog` job that runs `orhun/git-cliff-action@v4` with `--latest --strip header` over a full-history checkout, creates a draft release via `gh release create`, and outputs the release id; verify with `actionlint` or `gh workflow view` that the YAML parses
- [x] 3.2 Add the `build` matrix job (`macos-latest`, `windows-latest`, `fail-fast: false`) reusing the `ci.yml` setup steps and calling `tauri-apps/tauri-action@v1` with `releaseId` from the changelog job and `uploadUpdaterJson: false`; verify the YAML parses and the job needs `changelog`
- [x] 3.3 Add the `publish` job that needs `build` and runs `gh release edit --draft=false`; verify the YAML parses and that a failed `build` leaves `publish` skipped by reading the `needs` semantics in the file
- [x] 3.4 Update the README with the release procedure (dry run, `--execute`, manual override with `cargo release <level>`), the changelog conventions, and the one-time macOS `xattr -cr` step; verify the text matches the task and flags that actually exist after 2.4

## 4. First release and end-to-end verification

- [x] 4.1 Commit everything above on `main` and let `ci.yml` pass; verify the run is green on both platforms with no change to `ci.yml`
- [x] 4.2 Cut `v0.1.0` with `mise run release -- --execute` from one machine; verify one commit touching only `CHANGELOG.md`, `Cargo.toml`, and `Cargo.lock` exists, is tagged `v0.1.0`, and both are on the remote, and that `CHANGELOG.md` contains the full pre-release history as one section
- [x] 4.3 Verify the release workflow run: `changelog` created a draft named `Dango v0.1.0` whose body equals the `v0.1.0` section without the file header, both builds succeeded, `publish` ran, and the release is no longer a draft
- [ ] 4.4 Verify the Windows assets: download the `.msi` and the NSIS `.exe`, install one, confirm the installed application reports `0.1.0` and launches with the tray icon and hotkey working
- [x] 4.5 Verify the macOS asset: download the `.dmg`, run `xattr -cr` on the app, confirm it launches and `Info.plist` reports `0.1.0`
- [x] 4.6 Verify the nothing-to-release path after 4.2: `mise run release` on the freshly tagged `main` exits non-zero without creating a commit or tag
- [x] 4.7 Review the final diff for comments that only restate the YAML or TOML, for any leftover `version` in `tauri.conf.json`, and for scope creep such as updater or signing settings; delete what is found
