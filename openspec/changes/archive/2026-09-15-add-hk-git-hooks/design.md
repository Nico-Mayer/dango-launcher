## Context

See proposal.md for motivation. What shapes the approach:

- CI (`.github/workflows/ci.yml`) runs, in order: `npm run check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and a bindings diff on macOS. The Rust crate lives in `src-tauri/`, not at the root.
- Tooling is pinned in `mise.toml` (Node, OpenSpec). The author has mise on both machines, installed outside the repo.
- Git on the Windows machine is 2.55. Git 2.54 introduced config-based hooks, which hk prefers; on older Git (Apple's bundled Git, for instance) hk falls back to shims in `.git/hooks/`. Both are handled by `hk install`.
- No frontend formatter exists. Only rustfmt and clippy have a fix or check story today.
- hk 2.0.0 (released 2026-09-13) ships Windows x64 and arm64 builds, is in the mise registry as `aqua:jdx/hk`, and bundles its own Pkl evaluator, so no Pkl CLI is needed. Its builtins include `cargo_fmt` and `cargo_clippy`, both keyed on `workspace_indicator = "Cargo.toml"` so they find `src-tauri/Cargo.toml` from the root.

## Goals / Non-Goals

**Goals:**

- A developer who has run the two setup commands cannot push rustfmt drift or a clippy warning that CI would reject.
- Committing stays fast: only formatting runs at commit time.
- One config file, read by both the hooks and the on-demand `hk check` / `hk fix`.

**Non-Goals:**

- Anything the proposal lists as a non-goal, in particular making CI consume `hk.pkl`. Recorded below as a follow-up so the flag drift risk has a known exit.

## Decisions

### hk as the hook runner

Candidates checked: hk, lefthook, pre-commit (the Python framework), husky, hand-written `.git/hooks` scripts.

- **hand-written scripts**: no stash handling for partially staged files, no re-staging of fixes, one script per platform shell. Rejected as hand-rolled where a maintained tool exists.
- **husky**: runs through npm and would make the frontend package own hooks for a Rust-first repo. No fix-then-stage semantics, no stash. Rejected.
- **pre-commit**: mature, but needs a Python interpreter on both machines and its Rust hooks pull their own toolchain. Rejected for an extra runtime the project does not otherwise have.
- **lefthook**: Go binary, in the mise registry, Windows builds, parallel steps. Viable. It has no notion of check versus fix modes and no cargo builtins, so the same behaviour needs more hand-written config. Rejected in favour of hk, which the author also asked for.
- **hk**: single binary via mise, builtins for `cargo_fmt` and `cargo_clippy`, explicit check and fix modes, stashes unstaged work during a fixing pre-commit and re-stages the result, and `hk install --mise` makes hooks resolve mise tools without shell activation. Chosen.

### Format on commit, lint on push

Alternatives:

- Everything in `pre-commit`. Rejected: incremental `cargo clippy --all-targets` and `svelte-check` together add tens of seconds per commit on this crate. The user asked for a lightweight commit hook.
- Clippy in `pre-commit` using `--fix`. Rejected: `cargo clippy --fix` does not cover every lint that `-D warnings` denies and can change behaviour silently. Check mode with CI's exact flags is what stops CI failures.
- Nothing on push, only on demand. Rejected: the failure mode is forgetting to run it, which is the status quo.

Chosen: `pre-commit` runs `cargo fmt` in fix mode on staged Rust files and re-stages. `pre-push` runs clippy and `svelte-check` in check mode. `hk check` runs everything in check mode against the working tree, `hk fix` applies rustfmt.

### Mirror CI flags exactly, name steps after CI steps

The clippy builtin's default check is `cargo clippy --manifest-path {{workspace_indicator}} --quiet` with no `--all-targets` and no `-D warnings`. The step overrides `check` to add both, so a passing hook means a passing CI step. Step names in `hk.pkl` follow the CI step names (`Check formatting`, `Clippy`, `Type check frontend`) so a future edit to one is easy to mirror in the other.

Follow-up, not in this change: replace those three CI steps with `mise install` plus `mise exec -- hk check --all`, so the flags live in one place.

### svelte-check as a whole-project step

`svelte-check` is not a per-file tool. The step runs `npm run check` once, gated on a glob over `src/**`, `*.ts`, `svelte.config.js`, `tsconfig.json`, and `package.json`, so a Rust-only push does not pay for it.

### Pin hk in two places, same version

`mise.toml` pins `hk = "2.0.0"`. `hk.pkl` amends the schema package for the same version. Bumping means changing both lines in one commit.

### Local install, routed through mise

`hk install --mise` per clone, not `--global`. Global install requires Git 2.54 on every machine and touches the user's `~/.gitconfig`, which is outside this repo's remit. `--mise` wraps the hook in `mise x` so `cargo`, `node`, and `hk` resolve from PowerShell, lazygit, or an editor's git integration where mise is not activated.

### Sketch of `hk.pkl`

To be validated against the 2.0.0 schema during implementation; field names may need adjusting.

```pkl
amends "package://github.com/jdx/hk/releases/download/v2.0.0/hk@2.0.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v2.0.0/hk@2.0.0#/Builtins.pkl"

local fmt = Builtins.cargo_fmt

local clippy = (Builtins.cargo_clippy) {
  check = "cargo clippy --manifest-path {{workspace_indicator}} --all-targets -- -D warnings"
  fix = null
}

local svelteCheck = new Step {
  glob = List("src/**/*", "*.ts", "svelte.config.js", "tsconfig.json", "package.json")
  check = "npm run check"
}

hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps { ["cargo-fmt"] = fmt }
  }
  ["pre-push"] {
    steps {
      ["clippy"] = clippy
      ["svelte-check"] = svelteCheck
    }
  }
  ["check"] {
    steps {
      ["cargo-fmt"] = fmt
      ["clippy"] = clippy
      ["svelte-check"] = svelteCheck
    }
  }
  ["fix"] {
    fix = true
    steps { ["cargo-fmt"] = fmt }
  }
}
```

## Risks / Trade-offs

- [Flags in `hk.pkl` and `ci.yml` drift apart] → Step names mirror CI step names; the follow-up to have CI call `hk check --all` removes the duplication entirely.
- [Pre-push clippy does a full rebuild after a toolchain or dependency bump and takes minutes] → It is the same rebuild the next `cargo build` would do. `git push --no-verify` bypasses the hook for the rare case; CI still checks.
- [hk's Windows support is less exercised than macOS or Linux; the stash step in particular touches the index] → Verification tasks run the partially staged scenario on Windows first. If stash misbehaves on Windows, fall back to `stash = "none"` for pre-commit and accept that partially staged Rust files get fully formatted.
- [Hooks are opt-in per clone, so a forgotten `hk install` silently reverts to the old behaviour] → Accepted. CI remains the hard gate; the README documents the two commands.
- [The `--mise` launcher needs `mise` on PATH inside Git's hook shell on Windows] → mise is installed via winget and on the user PATH already. Verified as part of the Windows tasks.
- [`hk.pkl` fetches the schema package from GitHub on first evaluation] → hk caches it. An offline first run fails with a clear message; not a concern for this project.
