## 1. Purpose sections

- [ ] 1.1 Edit the Purpose sections a delta cannot change: replace "frontend" in `openspec/specs/view-protocol/spec.md`, and "a configured physical key" with CapsLock in `openspec/specs/hyperkey/spec.md`. Verify: `rg -n -i 'frontend|webview' openspec/specs` finds nothing outside requirements the deltas replace.

## 2. Check

- [ ] 2.1 Run `openspec validate sanitize-specs --strict`. Verify: it passes.
- [ ] 2.2 Read each delta against the code it cites and against design.md (spec follows code, bug rules #16-#22 kept, one copy per rule). Verify: no delta weakens a bug rule or adds behaviour the code does not have.
- [ ] 2.3 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context. The review only reports findings; the author decides what to fix.

## 3. Archive

- [ ] 3.1 Archive the change with `/opsx:archive`, which syncs the deltas into `openspec/specs/`. Verify: `rg -n -i 'frontend|webview2|webkit' openspec/specs` finds nothing, and `openspec validate --specs --strict` passes.
