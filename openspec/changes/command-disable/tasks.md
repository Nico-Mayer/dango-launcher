## 1. Host drops disabled commands

- [x] 1.1 Add `is_command_enabled(extension_id, command_id)` to `EnabledStore` with a default of `true`, and implement it in `FileConfig` from `extensions.<id>.commands.<id>.enabled` (absent means enabled); verify with a `cargo test` unit test in `src/config/store.rs` covering absent, true, and false
- [x] 1.2 Register only enabled commands when the host activates an extension; verify with a `cargo test` in `src/extension/mod.rs` that a disabled command is missing from `command_candidates` and `resolve_command` while a sibling command stays
- [x] 1.3 Make `reload_enabled` re-register the commands of extensions that stay active, without stopping their services; verify with a `cargo test` that disabling then re-enabling a command through the store updates the registry and a counting test service is started once

## 2. Hotkeys skip disabled commands

- [x] 2.1 Skip commands with `enabled: false` in `hotkeys::bindings`, keeping `disabled_with_hotkey`; verify with `cargo test` in `src/hotkeys.rs` that a disabled command gets no binding, and that a disabled and an enabled command on one chord give the enabled one the chord with no conflict

## 3. Wire-up and checks

- [ ] 3.1 Confirm config reload in `src/app/mod.rs` rebuilds candidates and hotkeys after the command re-sync, and add a `cargo test` with the recording fake shell that a command disabled by a reload is gone from root search and confirming a stale result reports "That command is no longer available"
- [ ] 3.2 Run `hk check` and `cargo test`; both pass
- [ ] 3.3 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context; it reports findings only and the author decides what to fix
