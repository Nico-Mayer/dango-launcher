# Proposal

## Why

When the launcher opens, it briefly shows old content before the current state appears. A window that is hidden is never repainted, so its first visible frame is the last frame drawn before the hide, for example the list or pushed view from before a reset. The window also appears at once with no transition, which makes the open feel abrupt. The author also wants an option to have the empty-query list ("Suggestions") open right away instead of opening it with Arrow Down.

## What Changes

- The launcher never shows content from before the hide. The window stays invisible until its first frame with the current state is drawn.
- The launcher opens with a short fade and a small downward slide of the pill and card. Typing and keys work during the animation.
- New config setting `launcher.open-list` (bool, default `false`). When `true`, a reset leaves the root view with the empty-query list open, as if the user had pressed Arrow Down. A held show (within the grace period) keeps the state it had, so the setting does not apply there. A fresh start of Dango also counts as a reset.

## Non-goals

- No closing (hide) animation. Hide stays instant, because paste, focus restore, and focus-loss hides depend on it.
- No animation of height changes, view pushes, or list changes.
- No setting for animation style, duration, or turning it off.
- No check of the system "reduce motion" option.
- No change to the Windows height handling or the macOS fixed window height.

## Platforms

- macOS: covered and verified in this session (the flash was seen here).
- Windows: gets the slide and the setting. The flash was not seen on Windows, and Slint notes that Windows discards what a hidden window drew, so a Windows fix is written only if a hand check in a Windows session shows the flash. Windows hand checks stay open tasks until that session, as AGENTS.md asks.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `launcher-shell`: new requirement saying the open shows no stale frame and has an opening animation. New requirement for the `launcher.open-list` setting. "Reset to root state on hide" now names the setting.
- `root-search`: "Empty query behaviour" says the root view starts closed unless `launcher.open-list` is on.

## Impact

- `src/platform/` (`LauncherWindow` trait, `macos`; `windows` only if the Windows hand check finds the flash): hide the window before show, reveal it after the first fresh frame.
- `src/slint_shell/mod.rs`, `src/slint_shell/render/`: reveal one loop turn after the `RedrawRequested` that draws the activated snapshot.
- `ui/shell/launcher-window.slint`, `ui/theme.slint`: slide animation driven by a `Shell` property.
- `src/config/mod.rs` (`Launcher`), schema, settings reference: new `open-list` key.
- `src/app/mod.rs`, `src/ui/state.rs`, `src/ui/input.rs`: reset and the initial state open the list when the setting is on.
- `AGENTS.md`: new platform traps and the renderer's new job.
- No change to the extension manifest or view protocol.
