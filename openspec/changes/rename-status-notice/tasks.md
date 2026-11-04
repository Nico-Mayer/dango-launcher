# Tasks

## 1. Tray problems

- [x] 1.1 Rename the tray's startup notices to problems: `TrayStatus::notices` and the startup `notices` list to `problems`, their tests (`notices_come_before_the_config_status`, `the_tray_lists_the_notices`), the comment at `src/app/mod.rs` ("raise no notice"), and "tray notice" in `interface-copy` and `global-activation` (requirement "Tray notices say what is wrong and where to look" included); `cargo test` passes and `rg -i "notices?" src openspec/specs` finds only prose verbs ("notice a focus change")

## 2. Specs and docs

- [ ] 2.1 In one commit: `git mv openspec/specs/hud openspec/specs/notice` (H1 included), apply the substring rules and the prose pass from design.md to every main spec, every file of `openspec/changes/launcher-v5-split-shell`, and AGENTS.md. Check: `rg -i "toast|hud" openspec/specs openspec/changes/launcher-v5-split-shell AGENTS.md` finds nothing; every `### Requirement:` title under `## MODIFIED Requirements` in the v5 deltas is found with `rg -F` in `openspec/specs/<capability>/spec.md`; `openspec validate --specs` passes with no new strict warnings (69 old ones on `main`), and `openspec validate launcher-v5-split-shell --strict` passes

## 3. Code

- [ ] 3.1 In one commit, apply the substring rules to `src/`, `ui/` and `tools/dango-ui-mcp/src/`: identifiers, Slint properties and enums, comments, the log line, test names; `git mv` `ui/shell/toast.slint` to `ticker.slint`, `ui/shell/hud.slint` to `notice.slint`, `src/slint_shell/render/hud.rs` to `notice.rs`. Check: `rg -i "toast|hud" src ui tools/dango-ui-mcp/src` finds nothing; `cargo test` and `mise run check` pass
- [ ] 3.2 macOS: CI's macOS job passes after the author pushes

## 4. Review

- [ ] 4.1 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context; report findings to the author

## Workflow follow-up

- Archive `launcher-v5-split-shell` at the top of the stack, after this branch.
