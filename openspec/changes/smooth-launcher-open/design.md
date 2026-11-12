# Design

## Context

`SlintShell::show` (`src/slint_shell/mod.rs`) calls `window.show()` and the platform's `did_show`. After that, `Core::show` applies `Event::Activated`. Each snapshot reaches the window through `publish`, which goes through `invoke_from_event_loop`. On macOS a hidden window is not repainted, so it is ordered front with its last surface from before the hide, and the fresh state shows one frame later. That is the flash.

On Windows the flash has not been seen. Slint's winit adapter notes that "X11 and Windows discard what is drawn into a window that isn't mapped" (`winitwindowadapter.rs`), so Windows may not need a fix. This is not verified.

Facts from reading Slint 1.18 source, which shape the design:
- Dango's `on_winit_window_event` filter runs before Slint handles the event (`event_loop.rs`). On `RedrawRequested`, the frame is drawn after the filter returns.
- On the first show, Slint draws the first frame before it maps the window. On macOS, `RevealOnFirstFrame` (`macos.rs`) saves the window alpha, sets it to 0, and restores the saved value after the first presented frame. So the first show has no old frame.
- `hide` calls `Renderer::forget_images`, which clears the list models. A held show would therefore also draw an empty list first, not only a reset.

The root view's open state is `root_open` in `src/ui/state.rs`. `reset()` sets it to `false`.

## Goals / Non-Goals

**Goals:**
- No visible frame from before the hide.
- Short opening animation that never blocks input.
- `launcher.open-list` setting that decides how the root view looks after a reset.

**Non-Goals:** see proposal.

## Decisions

### 1. Hide the window before show, reveal it after the activated frame is drawn

`LauncherWindow` gets two methods with no-op defaults:
- `conceal(window)`: called in `SlintShell::show` before `window.show()`, only when a native window already exists. On the first show there is none, and Slint's own first-frame handling covers it. macOS: `setAlphaValue(0.0)`.
- `reveal(window)`: macOS: `NSAnimationContext::runAnimationGroup` with `window.animator().setAlphaValue(1.0)`, which is the fade.

Windows keeps the no-op defaults. Only if the Windows hand check finds the flash does Windows get `DwmSetWindowAttribute(DWMWA_CLOAK)` (`TRUE` in `conceal`, `FALSE` in `reveal`).

When to reveal:
1. `Renderer` gets one bool, `reveal_pending`. `place_focus` already detects the activated snapshot (`render/mod.rs`); it sets the bool there and calls `window.request_redraw()`, so a redraw always follows even when the snapshot changes nothing.
2. The `RedrawRequested` handler takes the bool. If it was set, it queues the reveal with `slint::Timer::single_shot(Duration::ZERO, ...)`. The filter runs before Slint draws, so revealing in the handler itself would show the old surface for one frame.
3. The queued reveal calls `Renderer::reveal` (sets `Shell.revealed`) and `platform.reveal(handle)`, and does nothing if the launcher was hidden in between.

`did_show` still runs right after `window.show()`, so focus and keyboard are taken at once and typing during the fade works.

**Rejected: blank the window before hide** (set Slint opacity to 0, wait one frame, then hide). It delays every hide by one frame. Paste (`hide_for_work`), focus restore, and focus-loss hides depend on the window being gone at once.

**Rejected: repaint while hidden.** AppKit does not draw ordered-out windows, and Slint has no API to force a hidden window to render into its surface.

**Rejected: Slint opacity fade on the whole content.** The software renderer (Windows without OpenGL) does not support layer opacity (Slint docs), so overlapping children would show through each other during the fade. It also does not fix the old frame.

**Rejected: native fade on Windows via `SetLayeredWindowAttributes`.** The launcher is not a layered window. Making it one changes how it draws and how clicks pass through (see the layered-window trap in AGENTS.md).

The first show has no fade and no slide: `Shell.revealed` starts `true` and `conceal` is skipped, so Slint's own first-frame handling stays in charge (see Risks). The spec says so.

`conceal` sets alpha 0 through the animator with duration 0, not with a plain `setAlphaValue`: a fade still running from the last show would otherwise go on raising alpha before the fresh frame is drawn.

**Rejected: a three-state reveal enum.** One bool set where the activated snapshot is placed is enough.

**Rejected: write the Windows cloak now.** The flash was not seen on Windows, CI can only compile it, and cloaking risks `force_foreground` and `take_keyboard`.

### 2. Slide in Slint, driven by one `Shell` property

`Shell` global gets `in-property <bool> revealed: true`. The default is `true`, so `render_headless` and existing UI tests, which never go through `SlintShell::show`, render at rest. The renderer is its only writer: `Renderer::hide` (called from `SlintShell::hide`, next to `forget_images`) sets it to `false`, and `Renderer::reveal` sets it to `true`.

In `ui/shell/launcher-window.slint`, the pill and card container uses a `states` block: offset `-Theme.reveal-offset` when not revealed, `0` when revealed, with an animation only on the transition into revealed. Moving back to the offset on hide is instant, so the next show always starts from the full offset. A plain `animate y` would also animate the hide and leave the content mid-way at the next show.

`Theme` gets `reveal-offset` (a few logical pixels, within the 32 px shadow margin) and `reveal-duration`. The macOS fade uses the same duration through one Rust constant next to `LAUNCHER_SHADOW_MARGIN`; change both together, as with row heights.

Only `y` moves, so every renderer draws it correctly. No design-system part is added: this is a property of the existing launcher window shell part (`ui/shell/launcher-window.slint`). Fixed heights and the counting in `render/` are not affected, because the offset moves content, not the window.

### 3. `launcher.open-list` decides the state after a reset

- `config::Launcher` gets `open_list: Option<bool>` with doc comment (settings reference text):
  > Open the list of suggestions each time the launcher starts fresh, as if you pressed Down. Default false.
- `Event::Reset` becomes `Event::Reset { open_list: bool }`. `reset()` sets `root_open = open_list` instead of `false`. No helper is needed: `reset()` clears the results, so the Arrow Down path would only set `root_open` anyway, and the first row is selected by default.
- `Core::settle_hide` and the hold timeout read the setting from config at that moment, so a changed value applies at the next reset without a restart.
- The initial state: `Core::new` builds the state with `ui::Launcher::starting(open_list)`, which matches a reset. The first search still comes from `Intent::Ready`.
- A held show does not reset, so the setting never applies there.

**Rejected: keep the setting in `ui::Launcher` state.** The reducer would hold a copy of config that can go stale. Passing the value in the event keeps the reducer pure and matches how `Activated` carries its inputs.

**Rejected: apply on every show.** The author asked for reset only. A held show must keep what the user left (see launcher-shell).

### Contracts

No change to the extension manifest or view protocol. The config file gains one key.

## Risks / Trade-offs

- [The activated snapshot changes nothing that Slint redraws] -> `place_focus` requests a redraw itself, so the reveal always comes.
- [The macOS window server shows the alpha change before the GL frame is presented] -> the zero-delay timer runs after the draw; the hand check (5.1) confirms no old frame. If one still shows, move the reveal to the next redraw instead.
- [Show again during a running fade] -> `conceal` sets alpha 0 directly. Whether that stops a running `animator()` animation is not verified; the hand check covers fast hide and show.
- [Slint's `RevealOnFirstFrame` restores a saved alpha of 0 if Dango concealed first] -> `conceal` never runs on the first show.
