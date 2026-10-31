## Context

After `move-ui-state-to-rust`, the interaction rules live in a pure state
machine (`src-tauri/src/ui/`). The webview sends `Intent`s through one sync
`dispatch` command and draws the `Snapshot` from one `dango://state` event.
What still ties Dango to Tauri:

- `lib.rs` (about 1,800 lines): app setup, 6 plugins, the tray and its notice
  items, window show and hide, `run_on_main_thread`, the hold timer on
  `tauri::async_runtime`, and 32 `app.state::<T>()` lookups. `AppHandle` is
  the service locator for the whole backend.
- `platform/windows/mod.rs` and `platform/macos/mod.rs` reach the native
  window through `WebviewWindow::hwnd()` and `ns_window()`. macOS reclasses
  the window as an NSPanel and sets its level and collection behaviour.
  Windows sets `WS_EX_TOOLWINDOW` after each show.
- Plugin types leak into the core: `tauri_plugin_global_shortcut::{Shortcut,
  Modifiers, Code}` in `config/` and `hotkeys.rs`, `tauri::Url` in snippets,
  `tauri_plugin_updater` in `extensions/updates/source.rs`, the autostart
  manager in `autostart.rs`.
- Tests: 653 Rust tests, 64 Playwright tests through a cargo example fixture
  server. `tests/browser/README.md` marks 6 of them "slint", the rest "dies".

Spike results (Windows, release build, femtovg renderer): tray, hotkey, and a
50-row list with SVG icons idle at about 4 to 5 MB. The spike window did not
take keyboard focus when shown from the hotkey.

## Goals / Non-Goals

**Goals:**

- Dango runs as one process with a Slint window and no webview, on Windows
  and macOS.
- The state machine, search, extensions, and view protocol trees do not
  change.
- Every step on the way leaves a runnable app.
- The agent workflow (rules, skills, MCP, `AGENTS.md`) matches the new stack
  by the end of the change.

**Non-Goals:**

- Visual polish, a light palette, motion, acrylic or vibrancy.
- Packaging, signing, the updater, releases. See the proposal for the gap.

## Decisions

### 1. Core context and a `Shell` trait before any Slint code

`AppHandle` is replaced by an `Arc<Core>` that owns what `app.state()` serves
today: the extension host, the launcher state machine, the hold, the config,
the stores, and the platform services. Window and UI work goes through a
narrow trait:

```
trait Shell: Send + Sync {
    fn publish(&self, snapshot: Snapshot);
    fn show(&self, activation: Activation);
    fn hide(&self);
    fn on_main_thread(&self, work: Box<dyn FnOnce() + Send>);
    fn set_tray_notices(&self, notices: Vec<String>);
    fn quit(&self);
}
```

Tauri implements it first, so the refactor lands while the webview still
renders. The Slint shell implements it next, and both shells run off the same
core until the Tauri one is deleted. Tests use a recording fake.

Rejected: writing the Slint shell straight against a copy of `lib.rs`. Two
diverging copies of 1,800 lines, with no runnable state between "Tauri works"
and "Slint works".

### 2. Threads: Slint owns the main thread, tokio runs beside it

- The Slint event loop runs on the main thread. AppKit needs window work
  there, and `global-hotkey` and the tray need a message loop on the thread
  that registered them.
- A multi-thread tokio runtime is created at startup and owned by `Core`,
  replacing `tauri::async_runtime`. The hold timer and effects spawn on it.
- Backend to UI: `slint::invoke_from_event_loop`. This replaces
  `run_on_main_thread` and keeps the "every show and hide on the main
  thread" rule.
- UI to core: Slint callbacks call `Launcher::apply` directly and in order on
  the main thread. This keeps the reason `dispatch` was sync (keystrokes must
  not swap places) without a command layer.
- Key handling is now synchronous in-process. A key callback asks Rust
  whether the chord is in the snapshot's `bindings` and gets the answer back
  before Slint routes the key on. The binding table stays, as it does not
  need changing.

### 3. Renderer: generated Slint models from the snapshot, full replace

Each published snapshot is converted into Slint structs and set on the
component in one go: `VecModel::set_vec` for rows, plain properties for the
rest. No diffing, which matches the full-tree rule. Query and form text follow
the revision rule from `move-ui-state-to-rust`: the input is written only when
the revision changes.

Surfaces and their Slint building blocks (`std-widgets.slint`, Fluent style
forced on both platforms so the look is the same):

| Surface | Built on | Patched or hand-rolled, and why |
| --- | --- | --- |
| Root prompt, list filter | `LineEdit` | Key events go through the bindings callback first |
| Result list, pushed list, action list | `ListView` with a hand-made 56 px row | `StandardListView` has no icon, subtitle, tint, or highlight slots |
| Section headings | Plain `Text` rows in the same `ListView` | No grouped list widget exists |
| Action panel | `PopupWindow` | Opened and closed from the snapshot, not by click-outside |
| Detail | `ScrollView` + `Text` with wrapping | Same as today: the markdown is shown as plain text |
| Form fields | `LineEdit`, `TextEdit` (template), `CheckBox` | None |
| Actions button | `Button` | None |
| Footer, frame, dev marker, busy indicator | Plain `Rectangle`, `Text`, `Image` | Static, nothing to reuse |

The `PopupWindow` patch: its own close-on-click-outside is turned off
(`close-policy: no-auto-close`), because the state machine decides when the
panel closes. Otherwise a click could close it without the state machine
knowing, and the overlay keyboard-ownership rule would break.

Scroll-into-view: rows have fixed heights (56 px rows, fixed heading
height), so the renderer computes the selected row's offset and sets the
`ListView`'s `viewport-y` only when the row is outside the visible range.

Hover: rows paint hover from a `TouchArea`'s `has-hover`. A key press sets a
`pointer-resting` flag that hides hover until the pointer moves again, the
same rule `input.svelte.ts` implements today. Hover never selects.

Rejected:

- **Load `.slint` at runtime with `slint-interpreter`.** Slower to start,
  larger, and nothing here needs runtime-loaded UI. `slint-viewer
  --auto-reload` already gives fast iteration during development.
- **egui or gpui.** Compared in exploration: egui redraws continuously and
  has weaker IME, gpui's API is unstable. Slint was the measured option.

### 4. Window and platform

- The window is created hidden at startup with `no-frame`, transparent
  background, `always-on-top`, fixed size, and the same 720 x 400 size.
- `LauncherWindow` and the Windows and macOS implementations take a
  `raw_window_handle::RawWindowHandle` instead of a `WebviewWindow`. The
  handle comes from `i_slint_backend_winit::WinitWindowAccessor`. On macOS
  the NSView's `window` gives the NSWindow, which is reclassed as an NSPanel
  as today.
- macOS activation policy: set to accessory through
  `BackendSelector::with_winit_event_loop_builder` and winit's
  `with_activation_policy`, so no dock icon ever shows.
- Windows: `WS_EX_TOOLWINDOW` is re-applied after each show, as today. winit
  may rewrite the extended style like tao did. That is checked before
  assuming it.
- Focus: the spike's window did not take focus. The existing foreground-lock
  hand-off (`AttachThreadInput`, `AllowSetForegroundWindow`) runs on the HWND
  after show, then the renderer focuses the input from `snapshot.focus`.
- Positioning stays in `platform/`, physical pixels on Windows, logical
  points on macOS, now applied through winit's `set_outer_position`.
- Renderer: Slint's default (femtovg on OpenGL). The software renderer is the
  fallback if transparency or startup misbehaves on a machine. Both are
  compiled in, so `SLINT_BACKEND=winit-software` switches without a rebuild.
- Latency: `Window::set_rendering_notifier` with `AfterRendering` replaces
  `report_paint`, so `DANGO_MEASURE=1` keeps working.

### 5. Plugin replacements

| Tauri plugin | Replacement | Note |
| --- | --- | --- |
| global-shortcut | `global-hotkey` | The plugin wraps it. `HotKey`, `Modifiers`, `Code` are the same types, so `config/hotkey.rs` only changes imports |
| tray (core) | Slint `SystemTrayIcon` | Built in since 1.18. The menu's notice items come from a model with `for` |
| autostart | `auto-launch` | The plugin wraps it. Same login entries |
| single-instance | `single-instance` + `interprocess` | `single-instance` only detects a second launch. A local socket (named pipe on Windows) tells the first instance to show |
| opener | `open` | Opens URLs and paths with the system default |
| updater | none in this change | `UpdateSource` gets a `Disabled` source that never finds an update. `native-release-pipeline` brings a real one |

`tauri::Url` in snippets becomes `url::Url`, which is what it re-exports.

### 6. Icons

Lucide SVGs for every name in today's `Icon.svelte` registry are vendored
under `ui/icons/` and embedded with `include_bytes!`. One Rust registry maps
`icon:<name>` to an `Image` built with `Image::load_from_svg_data`, cached on
first use. Tints become a `colorize` colour on the `Image`. Paths still load
with `Image::load_from_path`, cached the same way, which replaces the asset
protocol.

Candidates checked during apply before vendoring (off the shelf first):
`icondata_lu` or another crate shipping Lucide SVG data. Vendoring wins
unless a crate gives the raw SVG for one icon without pulling a UI framework.

### 7. Layout

One crate at the repo root: `Cargo.toml`, Rust in `src/`, `.slint` in `ui/`,
`build.rs` compiling `ui/app.slint`. This move happens last, after Svelte has
left `src/`, so the two never share a directory. `git mv` keeps history.

### 8. Tests

- The 6 tests marked "slint" move to `cargo test` with
  `i_slint_backend_testing::init_no_event_loop`. They find elements with
  `find_by_element_id` or `find_by_accessible_label`, click with
  `mock_single_click`, and send keys through `Window::dispatch_event`. Each
  needs accessible labels or element ids in the `.slint` files.
- The dev-marker tests mount the component in both states (spec delta for
  `development-indicator`).
- Everything marked "dies" is deleted with Playwright, the fixtures, the
  `ui_fixture` example, and `tiny_http`.

### 9. Agent workflow

- First commit: the `openspec/config.yaml` rules name Slint std-widgets
  instead of Bits UI primitives, verify with `cargo test` instead of
  `npm run test:browser`, and do manual checks in a debug build instead of
  `tauri dev`. The rest of this change's artifacts already follow them.
- The official Slint skill installs with `npx skills add slint-ui/ai-plugins
  --skill slint`, so `skills-lock.json` restores it. `.mcp.json` adds
  `slint-docs` at `https://docs.slint.dev/mcp`, plus `dango-ui` at
  `http://localhost:9315/mcp` for a running debug build.
- The Slint app MCP needs `--features slint/mcp` on the command line and
  `SLINT_MCP_PORT`. It is never enabled in `Cargo.toml`, so a release build
  cannot open the port. `AGENTS.md` records the debug command.
- Last commit: the 51 `tauri*` skills and `svelte-code-writer` are removed
  from `.claude/skills` and `skills-lock.json`.
  `.claude/settings.json` sets `"svelte@svelte": false` under
  `enabledPlugins`.
- `AGENTS.md` is rewritten for the new stack:
  - stack, conventions, and the icon rule
  - Slint traps found during apply
  - build facts: no more `npx tauri build`, asset protocol, `tauri-action`,
    updater pubkey, or fixture server
- Node stays in `mise.toml`: the OpenSpec CLI and `scripts/release.mjs` use
  it. `package.json` and `node_modules` go.

### 10. Contracts

- View protocol trees and the extension manifest: unchanged.
- `Snapshot` and `Intent` become a Rust-only interface. `ts-rs` and
  `src/protocol/*.ts` are removed.
- Config: no change is needed. Free to break if one appears.

### 11. Strings

No string is added or changed. The Slint files take the existing strings
verbatim from the Svelte components and the snapshot: "Search apps and
commands", "Search", "Actions", "Open", "Select", the tray items, and the
"dev" badge. The README badge is not interface copy.

## Risks / Trade-offs

- **Focus on activation.** Seen failing in the spike. If the foreground-lock
  hand-off on the winit HWND is not enough, the fallback is the same trick
  Dango uses for paste, a synthetic key press before `SetForegroundWindow`.
  This is checked first on Windows, before the renderer surfaces are built.
- **`WS_EX_TOOLWINDOW` and winit.** winit may rewrite the extended style on
  visibility changes, like tao. If so, the existing re-apply after show
  covers it, and the trap in `AGENTS.md` is updated to name winit.
- **Transparent window on Windows with femtovg.** If it draws a black
  background, switch to the software renderer for that build, and record it
  as a trap.
- **IME.** Slint's `LineEdit` supports IME through winit. It is checked by
  hand with a Japanese IME on Windows and macOS. Not a blocker for the
  author's own use.
- **NSPanel reclass under winit.** winit owns its NSWindow subclass.
  Reclassing it to an NSPanel subclass could conflict with winit's delegate.
  The fallback is to keep the window class and only set level, collection
  behaviour, and `hidesOnDeactivate`, which covers fullscreen spaces for a
  borderless window. Checked by hand on the Mac early.
- **Release gap.** No updates or releases until `native-release-pipeline`.
  Accepted: the only user installs by hand.
- **Slint licence.** Royalty-free 2.0 needs attribution. The README badge is
  the chosen option, because the README is the download page.
