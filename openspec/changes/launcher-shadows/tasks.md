# Tasks

## 1. Shared shadow tokens

- [ ] 1.1 Add `shadow-blur` (24px), `shadow-offset` (8px), `shadow-margin` (32px) to `Theme` in `ui/theme.slint`, and switch `ActionPanel` to `shadow-blur` / `shadow-offset`. Verify `mise run check` and `cargo test` pass unchanged

## 2. Margin and shadows in the launcher window

- [ ] 2.1 Add `LAUNCHER_SHADOW_MARGIN` (32.0) next to `LAUNCHER_MAX_HEIGHT` in `src/platform/mod.rs`. In `LauncherWindow`, make the window `window-width + 2 * shadow-margin` wide, place pill and card at `x: shadow-margin` with `width: window-width`, move them down by the margin, and clamp the card from the launcher height instead of `root.height`. In `set_launcher_height`, write `height + 2 * LAUNCHER_SHADOW_MARGIN` to `Shell.height` and `platform.set_height`. Update the UI tests' click coordinates and pill/card positions by the margin; add a UI test that a click in the margin beside the card is `ClickedOutside`, and one that pill and card are `window-width` wide at `x` = margin. Verify `cargo test`
- [ ] 2.2 Give pill and card the shadow (`shadow-blur`, `shadow-offset`, `Theme.shadow`). If the card's `clip: true` cuts its shadow, move the shadow to a parent `Rectangle`. Verify with `dango-ui` `take_screenshot`: pixels below pill and card fade from the shadow to transparent before the window edge, in dark and light theme
- [ ] 2.3 Windows: use `LAUNCHER_MAX_HEIGHT + 2 * LAUNCHER_SHADOW_MARGIN` as the placement height in `position_on_active_display` (`src/platform/windows/mod.rs`). Verify the `launcher_origin` unit tests in `src/platform/mod.rs` still pass and `mise run check`

## 3. Hand checks

- [ ] 3.1 Windows: with `dango-ui` and `mise run shot`, check pill and card shadows over a light and a dark app, SearchBar stays at the same screen place as before the change (compare with a `shot` from `main`), height follows results without flicker, a real click in the margin and in the gap hides the launcher
- [ ] 3.2 macOS: check pill and card shadows, placement centred on the full frame at the same place as before, window stays full height with the area below the card's shadow transparent, a click in the margin hides the launcher
- [ ] 3.3 Add any platform trap found in 3.1 or 3.2 to `AGENTS.md`

## 4. Design file

- [ ] 4.1 Update `.design/Launcher Component System v5.dc.html` (gitignored): pill and card get `box-shadow: 0 8px 24px var(--shadow)`, drop the "no shadows" notes. Verify by rendering it with headless Edge and comparing with a `dango-ui` screenshot

## 5. Review

- [ ] 5.1 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context; report findings to the author

## Workflow follow-up

- Archive with `/opsx:archive` only after both platforms' hand checks are done.
