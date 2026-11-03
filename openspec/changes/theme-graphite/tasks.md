# Tasks

## 1. Sizes from the design

- [x] 1.1 In `ui/theme.slint`, set `row-height` to 44 px, `search-height` to 64 px, `action-bar-height` to 44 px, and `list-padding` to 8 px. In `src/slint_shell/render/height.rs`, set `ROW` to `44.0`, `SEARCH_BAR` to `64.0`, `ACTION_BAR` to `44.0`, and `LIST_PADDING` to `8.0` in the same commit. Verify with `cargo test` after the size tests in 1.2 pass.
- [x] 1.2 Update the size tests in `src/slint_shell/tests.rs`: rename `a_row_is_forty_pixels_high` to `a_row_is_forty_four_pixels_high` and expect 44, and rewrite the window height sums (`56.0 + 6.0 + n * 40.0 + 6.0 + 40.0`, around line 1410) to `64 + 8 + n * 44 + 8 + 44`. Check that `LIST_BOTTOM` and the `detail_bottom - 40.0` asserts (ActionBar height, not row height) follow the new ActionBar height of 44. Verify with `cargo test`.

## 2. Theme presets and new roles

- [x] 2.1 In `ui/theme.slint`, add a private `ThemeColors` struct with every colour role from design.md's mapping table, private `graphite` and `paper` properties on `Theme` with values from the design's `THEMES` table (rgba as 8-digit hex; `shadow` as in design.md), a private `colors` property that picks one by `dark`, and one `out` property per role. Remove every old colour role in the same commit. Verify with `cargo build`: it fails until 2.2 is done, so 2.1 and 2.2 land as one commit.
- [x] 2.2 Move every colour read in `ui/atoms`, `ui/shell`, and `ui/views` to the new roles, following design.md's mapping table: `foreground` for text, `popover` for the launcher window, `chrome` for the ActionBar, `input` for control borders, `dim` for placeholders, `selected` for selected rows, tiles, actions, and filter options, and `selection` only for text selection. Give `ui/atoms/tag.slint` a `with-alpha(0.18)` fill of its role and drop its outline. Move `ui/atoms/dot.slint` to `working` (busy), `done` (success), and `danger` (failure). Move the HUD pill in `ui/shell/hud.slint` to `popover` with a `border` line. Verify with `mise run check`, which compiles every `.slint` file and rejects any old name.
- [x] 2.3 In `src/slint_shell/tests.rs`, rewrite `the_scheme_picks_the_theme` to check Graphite values under the dark scheme and Paper values under the light scheme for `popover`, `foreground`, and `accent`, and update `hover_is_weaker_than_selection` to compare `hover` with `selected`. Add a test that `Theme.selected` and `Theme.selection` differ in both schemes. Verify with `cargo test`.
- [x] 2.4 Update the tech stack note on `Theme` in `AGENTS.md`: one `Theme` global with herdr colour role names, whose values come from a private preset struct in `ui/theme.slint` (Graphite for dark, Paper for light). Verify by reading the section against `ui/theme.slint`.

## 3. Review

- [x] 3.1 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context. Report the findings to the author. Verify that the review's findings are listed in the session.

## Workflow follow-up

- This is change 1 of the design-system v2 stack. Create its branch with `git-spice` (`gs branch create theme-graphite`). Later changes (progress atoms, task registry, inline answer, component adoption) stack on it.
- Archive after review with `/opsx:archive`.
