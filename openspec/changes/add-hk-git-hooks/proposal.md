## Why

CI fails often on things a local tool could have fixed before the commit existed: rustfmt drift (`style: format the tree with rustfmt`, `style(clipboard): apply rustfmt`) and clippy warnings (`fix(platform): drop needless return`). Each failure costs a round trip through the two-platform matrix and a fixup commit. The project already pins its tooling with mise, so a git hook runner that mise installs, hk (https://hk.jdx.dev), fits without adding a second toolchain.

Milestone: follow-up to **M0 - shell**, which established the CI matrix. This change moves the cheap part of that gate onto the developer's machine.

## What Changes

- Add `hk` to `mise.toml` so the hook runner is pinned and installed with the other tools.
- Add an `hk.pkl` at the repository root that mirrors the CI checks:
  - `pre-commit`: run `cargo fmt` on staged Rust files, apply the fix, re-stage.
  - `pre-push`: run `cargo clippy --all-targets -- -D warnings` and `svelte-check` with the same flags CI uses.
  - `hk check` and `hk fix` run the same steps on demand against the working tree.
- Document the one-time setup after clone in the README: `mise install`, then `hk install --mise`.
- CI is unchanged. The hook is a local mirror of it, not a replacement.

## Capabilities

### New Capabilities

- `local-checks`: the checks a developer's machine runs before a commit and before a push, so CI does not fail on formatting or lints.

### Modified Capabilities

None.

## Platforms

Windows and macOS, both first class: the author commits from both. Linux is out of scope as usual.

## Non-goals

- Rewriting CI to call `hk check --all`. CI works today; one source of truth for the flags is nice but not worth adding mise to the runners now. Recorded as a possible follow-up in the design.
- Adding a frontend formatter. Nothing formats Svelte or TypeScript today and CI does not check it, so this is not a CI failure source. Picking one (prettier with the Svelte plugin, or oxfmt) and reformatting the tree is its own change.
- Running `cargo test` or the protocol bindings diff in a hook. `cargo test` regenerates the ts-rs bindings and CI checks they are in sync, but the test run is too heavy for a hook meant to stay lightweight. Can be added later as an opt-in hk profile.
- Commit message linting, TOML or JSON formatting, markdown linting. Nothing in CI checks these.
- Enforcing the hook. It is opt-in per clone by design; CI remains the hard gate.

## Impact

- New files: `hk.pkl` at the repository root.
- Modified: `mise.toml` (add `hk`), `README.md` (setup section).
- No application code, manifest, view protocol, or CI workflow changes.
- Developer machines need `mise install` once to pick up `hk`, then `hk install --mise` once per clone.
