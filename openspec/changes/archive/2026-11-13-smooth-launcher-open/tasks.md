# Tasks

## 1. `launcher.open-list` setting

- [x] 1.1 Add `open_list: Option<bool>` to `config::Launcher` with the doc comment from design.md, include it in `is_empty`, and regenerate the schema and settings reference. Verify: `cargo test` (schema and reference tests), plus a schema test that `open-list = "yes"` is rejected.
- [x] 1.2 Change `Event::Reset` to `Event::Reset { open_list: bool }` and set `root_open = open_list` in `reset()`. Verify: `src/ui/tests.rs` tests that a reset with `open_list: true` shows the sectioned list with the first row selected (with and without a pinned item), that Escape then closes it and a second Escape hides, and that `open_list: false` keeps the closed pill.
- [x] 1.3 Read the setting in `Core` for every reset (`settle_hide`, hold timeout) and apply one reset at startup after config load. Verify: `src/app/tests.rs` tests that a deliberate hide and an expired hold open the list when the setting is on, that a held show keeps a closed list, and that a config reload changes the next reset.

## 2. Reveal after the activated frame

- [x] 2.1 Add `Shell.revealed` (default `true`) and `Renderer` methods: `place_focus` sets `reveal_pending` and requests a redraw for the activated snapshot, `take_reveal`, `reveal` (sets `revealed`), `hide` (sets `revealed` false). Verify: UI tests in `src/slint_shell/tests.rs` that `revealed` is true for a headless render, false after `hide`, `take_reveal` is set only after the activated snapshot is rendered, and `reveal` sets it true.
- [x] 2.2 Add `conceal` and `reveal` with no-op defaults to `LauncherWindow`. In `SlintShell::show` call `conceal` only when a native window already exists; in `hide` call `Renderer::hide`; in the `RedrawRequested` handler queue the reveal with `slint::Timer::single_shot(Duration::ZERO, ...)` when `take_reveal` is set, skipping it if hidden by then. Verify: `cargo test` and `mise run check`.
- [x] 2.3 macOS: `conceal` sets `alphaValue` 0, `reveal` animates it to 1 with `NSAnimationContext` and the shared duration constant. Verify: `mise run check`.

## 3. Slide animation

- [x] 3.1 Add `Theme.reveal-offset` and `Theme.reveal-duration`, and a `states` block in `ui/shell/launcher-window.slint` that animates the pill and card `y` only into the revealed state. Verify: UI test that after `hide` the content sits at the offset at once, that it reaches its resting position after `reveal`, and that typing into the SearchBar before `reveal` still updates the query.

## 4. Docs

- [x] 4.1 Update AGENTS.md: platform traps (macOS hidden window keeps its last surface; the winit event filter runs before Slint draws; Slint's `RevealOnFirstFrame` restores the saved alpha on macOS first show) and the architecture line on what the renderer keeps (add the reveal). Verify: `mise run check` passes and the text names the files involved.

## 5. Hand checks

- [x] 5.1 macOS hand check with `dango-ui`: push a view, reset with Escape stages, show again: no frame of the old view, fade and slide visible, typing during the fade works, hide stays instant. Also: hold (focus loss) then show, show and hide fast several times inside the fade, first show after start, show over a fullscreen app.
- [x] 5.2 Windows hand check with `dango-ui`, `mise run keys`, and `mise run shot`: does the launcher show an old frame after a reset? If yes, add `DWMWA_CLOAK` in `conceal`/`reveal` (check symbol names in vendored `windows-sys`) and repeat. Then check slide, focus, and paste after a show (open until a session on a Windows machine).

## 6. Review

- [x] 6.1 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context. Report findings to the author; the author decides what to fix.

## Workflow follow-up

- Archive with `/opsx:archive` only after 5.2 is done on a Windows machine.
