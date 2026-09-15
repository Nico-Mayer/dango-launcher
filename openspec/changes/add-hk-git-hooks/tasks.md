## 1. Pin the tool

- [x] 1.1 Add `hk = "2.0.0"` to `mise.toml` and verify `mise install` completes and `mise exec -- hk --version` prints 2.0.0 on Windows

## 2. Write the config

- [x] 2.1 Create `hk.pkl` at the repository root from the design sketch, using the 2.0.0 schema, and verify `hk validate` (or `hk check --all` if validate is absent) evaluates the config without errors
- [x] 2.2 Introduce a deliberate formatting error in a Rust file and verify `hk check` exits non-zero and names the file, then verify `hk fix` rewrites it to rustfmt output and leaves it unstaged; revert the file afterwards
- [x] 2.3 Introduce a deliberate clippy warning (for example a needless `return`) and verify `hk check` fails on the clippy step with the same message CI would print; revert afterwards
- [x] 2.4 Introduce a deliberate type error in a Svelte component and verify `hk check` fails on the svelte-check step, and that `hk check` on a Rust-only change does not run svelte-check; revert afterwards

## 3. Install and verify hooks on Windows

- [x] 3.1 Run `hk install --mise` and verify the hook entries appear in `.git/config` (Git 2.54+), or `.git/hooks/pre-commit` exists on older Git
  - Git 2.55 on Windows, so hk wrote config-based hooks: `hook.hk-pre-commit`
    and `hook.hk-pre-push` in `.git/config`, each running
    `mise x -- hk run <hook> --from-hook` unless `HK=0` is set. `.git/hooks/`
    stayed untouched.
- [x] 3.2 From PowerShell, stage a misformatted Rust file plus an unstaged hunk in the same file, commit, and verify the commit contains rustfmt output while the unstaged hunk is still present and unformatted in the working tree
  - Verified in a scratch worktree on a scratch branch, committing from
    PowerShell. The commit contained rustfmt output for the staged hunk; the
    unstaged, misformatted function was still in the working tree afterwards
    and not in the commit. hk stashed the unstaged hunk with `git stash`,
    ran `cargo fmt`, re-staged `src-tauri/src/main.rs`, and restored the stash.
  - Finding: HEAD failed `cargo fmt --check` in 15 files before this change,
    so the first `hk fix` reformatted them all. `cargo fmt` formats the whole
    crate, not only staged files. A formatting commit has to land before the
    hook is pleasant to use; the formatted files were left in the working tree
    for review.
- [x] 3.3 Commit a Markdown-only change and verify the hook adds under 1 second (time the commit with and without the hook)
  - Markdown-only commits: about 480 ms with the hook, about 100 ms with
    `HK=0`. Hook overhead about 380 ms, of which none is Rust tooling.
- [x] 3.4 Commit a clippy warning, attempt a push to a scratch branch, and verify the push is aborted with the warning printed; then fix, push, and verify it goes through
  - A real `git push` was not run in this session (the session's permission
    policy blocked the command), so the hook was driven exactly as git drives
    it: `mise x -- hk run pre-push --from-hook <remote> <url>` with the ref
    line on stdin, against a local bare repository. With a needless `return`
    committed it exited 101 after 9 s with the same `-D clippy::needless-return`
    message CI prints. After dropping the `return` it passed in 3 s: clippy and
    svelte-check both green. Worth one real push to confirm git wires it up.
  - hk fails fast: when svelte-check failed first (no `node_modules` in the
    fresh worktree), clippy was aborted. Run `npm ci` before the first push in
    a fresh clone.
- [x] 3.5 Repeat 3.2 from lazygit or the editor's git integration to verify the `mise x` launcher resolves `cargo` without an activated shell
  - lazygit was not driven interactively. Simulated with the user's environment
    but PATH stripped of every mise and node entry, leaving only the winget
    `mise` launcher and `~/.cargo/bin`. The commit ran the hook, `cargo fmt`
    fixed the file, exit 0. With `env -i` (no HOME, no APPDATA) mise itself
    fails with "cannot find binary path"; editors and lazygit inherit the
    user environment, so that case does not apply.

## 4. Install and verify hooks on macOS

- [ ] 4.1 Run `mise install` and `hk install --mise` on the macOS machine and verify the hooks are registered (config entries on Git 2.54+, shim in `.git/hooks/` otherwise)
- [ ] 4.2 Repeat the partially staged Rust commit from 3.2 and verify the same outcome
- [ ] 4.3 Repeat the aborted push from 3.4 and verify the same outcome

## 5. Document

- [x] 5.1 Add a "Development setup" section to `README.md` with `mise install` and `hk install --mise`, one line on what each hook does, and `git push --no-verify` as the escape hatch; verify the commands work when followed verbatim in a fresh clone

## 6. Close out

- [ ] 6.1 Push the change and verify CI is green on both matrix jobs with `ci.yml` untouched
- [ ] 6.2 Record the Windows and macOS verification results from sections 3 and 4 in this change before archiving
