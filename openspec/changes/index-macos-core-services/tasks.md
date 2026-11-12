# Tasks

## 1. Index Finder and core services applications on macOS

- [ ] 1.1 In `src/platform/macos/apps.rs`, add `/System/Library/CoreServices/Applications` to `roots()`, and index `/System/Library/CoreServices/Finder.app` in `index()` behind the existing `is_bundle` guard (no new helper, not by walking `CoreServices`). Verify with the live test `the_installed_applications_are_found`, extended to also find Finder and Keychain Access and to find no `Dock.app`, and with `every_installed_application_has_an_icon` for the new bundles: `cargo test -- --ignored the_installed_applications_are_found every_installed_application_has_an_icon`
- [ ] 1.2 Run `mise run check` and `cargo test`. Both pass.

## 2. Manual check on macOS

- [ ] 2.1 With the debug build through `dango-ui`, type "Finder" and press Enter. Finder comes to the front.

## 3. Review

- [ ] 3.1 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context. Report findings to the author.
