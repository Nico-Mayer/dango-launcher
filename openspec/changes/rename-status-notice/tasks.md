# Tasks

## 1. Specs and docs

- [ ] 1.1 `git mv openspec/specs/hud openspec/specs/notice`, then reword every main spec (toast → status line, HUD → notice, tray notice → tray problem), including requirement titles and "see `hud`" links; `openspec validate --specs --strict` passes and `rg -i "\btoasts?\b|\bhud\b" openspec/specs` finds nothing
- [ ] 1.2 Reword the deltas of `openspec/changes/launcher-v5-split-shell` the same way, so they still match the renamed requirement titles; `openspec validate launcher-v5-split-shell --strict` passes
- [ ] 1.3 Reword AGENTS.md (platform traps naming the HUD); `rg -i "\bhud\b|toast" AGENTS.md` finds nothing

## 2. Code

- [ ] 2.1 Rename every identifier, file, comment, log line and test name in `src/` and `ui/` per the table in design.md, in one commit: `git mv` for `ui/shell/toast.slint`, `ui/shell/hud.slint`, `src/slint_shell/render/hud.rs`; `cargo test` and `mise run check` pass, and `rg -i "toast|hud" src ui` finds nothing
- [ ] 2.2 Windows: `cargo build` passes (covered by 2.1 on this machine)
- [ ] 2.3 macOS: check the renamed names in `src/platform/macos` against the vendored crates and the trait in `src/platform/mod.rs`; CI's macOS build passes

## 3. Tray problems

- [ ] 3.1 Rename `TrayStatus::notices` and the startup `notices` list to `problems`, with their tests (`notices_come_before_the_config_status`, `the_tray_lists_the_notices`); `cargo test` passes and `rg -w notices src` finds nothing

## 4. Review

- [ ] 4.1 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context; report findings to the author

## Workflow follow-up

- Archive after `launcher-v5-split-shell`, or before it: both keep matching specs.
