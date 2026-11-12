# Tasks

## 1. Open list as resting root view

- [ ] 1.1 Add `open_root` to `ui::Launcher`, set in `starting` and `reset`. In `escape`, clear a typed prompt to `root_open = open_root` and dismiss on an empty prompt unless the list is open and `open_root` is false; in `query_changed`, an emptied root query sets `root_open = open_root` and clears `picked` (design.md decision 1). Verify: `src/ui/tests.rs` tests that with `Reset { open_list: true }` one Escape on the empty prompt returns `Effect::Dismiss`; that Escape with text typed clears it and shows the sectioned list with the first row selected; that deleting the last character shows the sectioned list; and that `escape_clears_the_query_then_closes_the_list_then_dismisses` and `clearing_the_query_closes_the_list` still pass with `open_list: false`. Replace the Escape part of `a_reset_with_open_list_opens_the_list_on_the_first_row`.
- [ ] 1.2 Check `src/app/tests.rs` open-list tests: a held show test that relies on closing the list with Escape is rewritten to hold a typed prompt instead (spec scenario "A held show keeps its state"). Verify: `cargo test`.
- [ ] 1.3 Change the `open_list` doc comment in `src/config/mod.rs` to the text in design.md decision 3 and regenerate the reference with `env DANGO_UPDATE_DOCS=1 cargo test the_settings_reference_is_current` (and the schema, if its test asks). Verify: `cargo test` and `mise run check` pass.

## 2. Review

- [ ] 2.1 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context. Report findings to the author; the author decides what to fix.

## Workflow follow-up

- Archive `smooth-launcher-open` first: this change's deltas modify text it adds.
- No hand checks: the change is in the platform-neutral reducer only.
