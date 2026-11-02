# Tasks

## 1. Accessibility warning names the hyperkey

- [x] 1.1 Change the Accessibility subtitle in `turns_off` (`src/extensions/permissions/mod.rs`) to "Pasting, snippets, AI paste, window management, and the hyperkey don't work without it"; verify with a new test `the_accessibility_warning_names_the_hyperkey` beside `one_missing_permission_is_one_pinned_warning` (`cargo test`)
- [x] 1.2 Add a build fact to AGENTS.md: a local macOS bundle sharing `com.nimayer.dango` with the installed Dango gets confused with it in System Settings, and macOS's reopen after a grant starts the installed one. Test permissions with a copy under its own `CFBundleIdentifier`, ad-hoc signed with `codesign --sign -`, outside `/tmp`
- [x] 1.3 Run `mise run check` and `cargo test`

## 2. Verification

- [x] 2.1 macOS, hyperkey configured, Input Monitoring granted, Accessibility revoked while Dango runs: verify the Accessibility warning item shows the new subtitle, `dango.log` says "hyperkey: needs the Accessibility permission", CapsLock works normally, and typing and the mouse stay normal; grant it again and verify the hyperkey works within a few seconds without a restart. Test with a bundle under its own identifier (see AGENTS.md build facts) so the installed Dango's grants stay untouched
- [x] 2.2 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context
