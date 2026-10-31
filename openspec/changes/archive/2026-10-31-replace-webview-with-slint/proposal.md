## Why

Dango idles at about 181 MB of private memory on Windows, and about 139 MB of
that is the WebView2 process tree. A Slint spike with a tray, a hotkey, and a
50-row list idled at about 4 to 5 MB. Since `move-ui-state-to-rust`, the
frontend only draws a Rust snapshot and sends intents, so swapping the
renderer no longer touches any interaction rule.

## What Changes

- **BREAKING** Replace Tauri and the Svelte/WebView2 frontend with a native
  Slint renderer in the Dango process. It draws the same `Snapshot` and sends
  the same `Intent`s, and the state machine in `ui/` stays as it is.
- Replace the Tauri plugins:
  - global shortcut: the `global-hotkey` crate
  - tray: Slint's built-in tray icon
  - autostart: `auto-launch`
  - single instance: `single-instance`, plus a local socket so a second
    launch shows the first
  - opener: `open`
- Port the platform window code to the Slint window: NSPanel and accessory
  activation policy on macOS, `WS_EX_TOOLWINDOW` and focus hand-off on
  Windows, positioning on both.
- Icons come from `icondata_lu` behind one Rust icon registry.
- **BREAKING** Minimal styling: one dark palette, no light palette, no
  motion, no acrylic or vibrancy. Row and header heights stay the same.
- **BREAKING** The project layout changes: one crate at the repo root, Rust in
  `src/`, `.slint` files in `ui/`. `src-tauri/` goes away.
- Remove Svelte, Vite, Bits UI, Tailwind, Lucide for Svelte, `ts-rs`,
  Playwright, the fixtures, the `ui_fixture` example, `tiny_http`, and the
  npm toolchain. The browser tests marked "slint" in
  `tests/browser/README.md` move to the Slint testing backend.
- **Gap until `native-release-pipeline`:** the in-app updater and the release
  workflow are switched off. Builds are installed by hand meanwhile. The
  update item never appears, and no tag produces a release.
- Agent workflow:
  - `openspec/config.yaml` rules move from Bits UI and browser tests to Slint
    widgets and `cargo test`
  - the official Slint skill and the Slint docs MCP are added
  - the Slint app MCP is allowed in debug builds only
  - the Tauri skills and the Svelte tooling are removed
  - `AGENTS.md` is rewritten for the new stack
- The README carries the Slint attribution badge, which the Slint royalty-free
  license asks for. Dango stays MIT.

## Non-goals

- Visual polish, theme tokens, a light palette, transitions, acrylic or
  vibrancy. A later change brings the look back.
- Packaging, signing, installers, the updater, and the release workflow. That
  is `native-release-pipeline`.
- Changes to the state machine, search, extensions, the view protocol trees,
  or the extension manifest.
- A third-party extension host.
- Linux.

## Platforms

Windows and macOS, both fully. Every shell piece, plugin replacement, and
platform trap is ported on both. Windows window code is checked by hand on
Windows. macOS window code (NSPanel over fullscreen, accessory policy, focus)
is checked by hand on the Mac.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `launcher-shell`: add that the UI renders inside the Dango process, with no
  browser engine or helper process.
- `visual-system`: one dark palette, no motion, no theme overrides. Density,
  icon identity, readable states, overlay keyboard ownership, and fitting the
  viewport stay. Reduced motion goes, because nothing moves.
- `development-indicator`: the marker is drawn in the launcher window, not the
  webview, and both states are tested headless instead of in browser
  fixtures.
- `local-checks`: the push hook runs clippy, which also compiles the `.slint`
  files. The frontend type check goes away.
- `release-pipeline`: one version source without `package.json`.
- `ai-providers`: keys and requests never reach the UI layer (was: the
  webview).

## Impact

- Code: `src-tauri/src/lib.rs` (Tauri setup, 32 state lookups through
  `AppHandle`), Tauri imports in `hotkeys.rs`, `autostart.rs`,
  `config/mod.rs`, `config/hotkey.rs`, `platform/{mod,windows,macos}`,
  `extensions/updates/source.rs`, and `extensions/snippets/*`. All of `src/`
  (Svelte). `tests/browser`, `tests/fixtures`, `src-tauri/examples`.
- Dependencies: remove `tauri`, its 6 plugins, `ts-rs`, `tiny_http`, and all
  npm packages. Add `slint`, `slint-build`, `i-slint-backend-testing`,
  `icondata_lu`, `icondata_core`, `winresource`, `global-hotkey`, `auto-launch`, `single-instance`,
  `interprocess`, `open`, `url`, `raw-window-handle`.
- Tooling: `hk.pkl`, `.github/workflows/ci.yml`, `release.yml` (disabled),
  `mise.toml`, `package.json` (removed, unless the OpenSpec CLI needs Node
  through mise), `tauri.conf.json` (removed), `.mcp.json` (new),
  `.claude/settings.json` (new), `.claude/skills`, `skills-lock.json`.
- Docs: `AGENTS.md`, `README.md`.
- Specs not touched but suspended until `native-release-pipeline`:
  `app-updates`, and the release half of `release-pipeline`.
