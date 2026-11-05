# Tasks

## 1. Shared shadow tokens

- [x] 1.1 Add `shadow-blur` (24px), `shadow-offset` (8px), `shadow-margin` (32px) to `Theme` in `ui/theme.slint`, and switch `ActionPanel` to `shadow-blur` / `shadow-offset`. Verify `mise run check` and `cargo test` pass unchanged

## 2. Margin and shadows in the launcher window

- [x] 2.1 Add the margin, as design.md describes:
  - `LAUNCHER_SHADOW_MARGIN` (32.0) next to `LAUNCHER_MAX_HEIGHT` in `src/platform/mod.rs`.
  - `LauncherWindow`: `width: window-width + 2 * shadow-margin`, `height: Shell.height + 2 * shadow-margin`; pill and card inside one container `Rectangle` at the margin with no `TouchArea`; the card clamp becomes `min(Shell.launcher-height, parent.height) - self.y`.
  - `set_launcher_height`: `Shell.height` keeps the launcher height; only `platform.set_height` gets `height + 2 * LAUNCHER_SHADOW_MARGIN`.
  - Windows: `position_on_active_display` (`src/platform/windows/mod.rs`) uses `LAUNCHER_MAX_HEIGHT + 2 * LAUNCHER_SHADOW_MARGIN` as placement height.
  - Tests: harness and `render_headless` window size become 784 × 544; offset `LIST_TOP`, `LIST_BOTTOM`, the gap click (`MARGIN + 360, MARGIN + 56`), and other absolute-position asserts (panel `x`, grid bottom, ticker `y`) by the margin, or make them relative to the pill or card.
  - New UI tests: a click in the margin beside the card is `ClickedOutside`; pill and card are `window-width` wide at `x` = margin.
  - Verify `cargo test` and `mise run check`
- [x] 2.2 Give pill and card the shadow (`shadow-blur`, `shadow-offset`, `Theme.shadow`). Verify with `dango-ui` `take_screenshot`: pixels below pill and card fade from the shadow to transparent before the window edge, in dark and light theme

## 3. Hand checks

- [x] 3.1 Windows: with `dango-ui` and `mise run shot`, check pill and card shadows over a light and a dark app, SearchBar stays at the same screen place as before the change (compare with a `shot` from `main`), height follows results without flicker, a real click in the margin and in the gap hides the launcher
  - Agent part done, femtovg, 5120×2160 primary at 150 percent. Placement: window rect (96-DPI units) 1315,275 784 wide, so the pill sits at 1347,307; the old formula with 720×480 gives 1346.5,307.2. Same place, so no `main` build was needed.
  - Height: bare pill window 116 (52 + 64), a long list (`se`) 544 (480 + 64), one heading and one row 256 (192 + 64). Top edge stays at 275 throughout.
  - Real screen capture over a dark app: margin and gap transparent, app behind shows through the gap, shadow barely visible on the dark background as expected.
  - Shadow alpha from `take_screenshot`: fades to 0 before every window edge, light and dark theme (see 2.2).
  - Checked by the author by eye: shadow over a light app, no flicker while typing, a real click in the margin and in the gap hides the launcher.
- [ ] 3.2 macOS: check pill and card shadows, placement centred on the full frame at the same place as before, window keeps its full height with the area below the card's shadow transparent, the card's bottom and shadow are not cut, a click in the margin hides the launcher
- [ ] 3.3 Add any platform trap found in 3.1 or 3.2 to `AGENTS.md`

## 4. Design file

- [x] 4.1 Update `.design/Launcher Component System v5.dc.html` (gitignored): pill and card get `box-shadow: 0 8px 24px var(--shadow)`, drop the "no shadows" notes. Verify the file contains no "no shadow" note left

## 5. Review

- [x] 5.1 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context; report findings to the author

## Workflow follow-up

- Archive with `/opsx:archive` only after both platforms' hand checks are done.
