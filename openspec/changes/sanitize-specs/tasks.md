## 1. Purpose sections

- [x] 1.1 Edit the Purpose sections a delta cannot change: replace "frontend" in `openspec/specs/view-protocol/spec.md`, and "a configured physical key" with CapsLock in `openspec/specs/hyperkey/spec.md`. Verify: `rg -n -i 'frontend|webview' openspec/specs` finds nothing outside requirements the deltas replace.

## 2. Check

- [x] 2.1 Run `openspec validate sanitize-specs --strict`. Verify: it passes.
- [x] 2.2 Read each delta against the code it cites and against design.md (spec follows code, bug rules #16-#22 kept, one copy per rule). Verify: no delta weakens a bug rule or adds behaviour the code does not have.
- [x] 2.3 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context. The review only reports findings; the author decides what to fix.
