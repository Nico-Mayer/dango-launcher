Each numbered group is one git-spice branch stacked on the previous one, and
leaves a runnable app.

## 1. Agent workflow first

- [x] 1.1 Update `openspec/config.yaml` rules: design names the Slint widget per surface instead of the Bits UI primitive, tasks verify with `cargo test` (including the Slint testing backend) instead of `npm run test:browser`, manual checks run a debug build instead of `tauri dev`. Verify `openspec validate replace-webview-with-slint --strict` still passes.
- [ ] 1.2 Install the Slint skill with `npx skills add slint-ui/ai-plugins --skill slint` and decline the repo star prompt. Verify `skills-lock.json` lists `slint` and `.claude/skills/slint` resolves.
- [x] 1.3 Add `.mcp.json` with `slint-docs` (`https://docs.slint.dev/mcp`) and `dango-ui` (`http://localhost:9315/mcp`). Verify `claude mcp list` shows both from the project.

## 2. Plugin types out of the core

- [x] 2.1 Replace `tauri_plugin_global_shortcut` types in `config/mod.rs`, `config/hotkey.rs`, and `hotkeys.rs` with `global-hotkey` types. Verify `cargo test config:: hotkeys::` passes.
- [x] 2.2 Replace `tauri::Url` with `url::Url` in `extensions/snippets/*`. Verify `cargo test snippets::` passes.
- [x] 2.3 Add a `Disabled` `UpdateSource` and a test that it never reports an update. Verify `cargo test updates::`.

## 3. Core context and `Shell` trait (Tauri still renders)

- [ ] 3.1 Introduce `Core` owning the state behind today's 32 `app.state()` lookups, and a tokio runtime replacing `tauri::async_runtime`. Verify `cargo test` and clippy pass.
- [ ] 3.2 Introduce the `Shell` trait from design decision 1, with a Tauri implementation and a recording fake. Route snapshot publishing, show, hide, main-thread work, tray notices, and quit through it. Verify `cargo test` with tests that drive activation, hold expiry, and dismiss through the fake.
- [ ] 3.3 Change `LauncherWindow` and the Windows and macOS implementations to take a raw window handle instead of a `WebviewWindow`. Verify clippy on Windows locally and on macOS in CI.

## 4. Slint shell beside Tauri

The Slint shell is a second binary (`dango-slint`) on the same core, so the
Tauri app keeps working until group 6.

- [ ] 4.1 Add `slint` and `slint-build`, a `build.rs`, and `ui/app.slint` with the hidden borderless transparent window, and the `dango-slint` binary that starts `Core`, the Slint event loop, and the tokio runtime. Verify `cargo build --bin dango-slint`.
- [ ] 4.2 Tray via `SystemTrayIcon` with the toggle, quit, and notice items and the development tooltip. Verify with a `cargo test` that the tray model lists the notices and the tooltip reads "Dango (development build)" in a debug build.
- [ ] 4.3 Global hotkey via `global-hotkey`, registered on the main thread after the event loop starts, toggling through `Shell`. Verify `cargo test hotkeys::`.
- [ ] 4.4 Windows window: raw handle from `WinitWindowAccessor`, `WS_EX_TOOLWINDOW` after each show, positioning, foreground-lock hand-off. Verify by hand on Windows: the hotkey shows the window over a maximized app, the window takes typing, Alt+Tab does not list it.
- [ ] 4.5 macOS window: accessory activation policy through the winit event loop builder, NSPanel reclass or the fallback from the design risks, level and collection behaviour. Verify by hand on macOS: no dock icon, the window appears over a fullscreen app without switching spaces, and takes typing.
- [ ] 4.6 Single instance with `single-instance` and an `interprocess` local socket that shows the running launcher. Verify with a `cargo test` that a second instance's message reaches the first.
- [ ] 4.7 Autostart through `auto-launch` and opening URLs and paths through `open`. Verify `cargo test autostart::` and the snippets opener tests.
- [ ] 4.8 Icon registry: check `icondata_lu` and similar crates first, else vendor the Lucide SVGs named in `src/lib/Icon.svelte` into `ui/icons/`, and load path icons with a cache. Verify with a `cargo test` that every name in the old registry resolves to an image and an unknown name falls back.

## 5. Slint surfaces

- [ ] 5.1 Palette global with the dark values, frame, footer, dev marker, and busy indicator. Verify the two `development-indicator` tests (dev state shows marker and badge, release state shows neither) with `i_slint_backend_testing`.
- [ ] 5.2 Root search: prompt with the revision rule, sections, 56 px rows with icon, tint, title highlight, and subtitle, empty message, scroll-into-view, hover with the resting-pointer rule. Verify the ported "slint" tests from `navigation.spec.ts` (hover never selects, clicking keeps the typed query, a later group's first row scrolls into view) with `cargo test`.
- [ ] 5.3 Pushed list, detail, and form views, including the template field and its error. Verify with `cargo test` that each view kind renders from a snapshot and the form error is the field's accessible description.
- [ ] 5.4 Action panel on `PopupWindow` with auto-close off, its own list, and shortcuts shown. Verify with `cargo test` that the panel opens and closes only from the snapshot and a click outside does not reach the parent.
- [ ] 5.5 Protocol error screen and the dev marker on every surface. Verify the third `development-indicator` test (error screen keeps the edge) with `cargo test`.
- [ ] 5.6 Latency notifier with `set_rendering_notifier`, so `DANGO_MEASURE=1` prints hotkey-to-paint times. Verify by hand in a release build on Windows.

## 6. Switch and delete

- [ ] 6.1 Make the Slint shell the `dango` binary and delete the Tauri shell, `tauri.conf.json`, the Tauri crates, `ts-rs`, `src/protocol/*.ts`, `tiny_http`, and `examples/ui_fixture.rs`. Verify `cargo test` and clippy pass and `cargo tree` has no `tauri` or `wry`.
- [ ] 6.2 Delete Svelte, Vite, `src/` frontend files, `index.html`, `package.json`, `package-lock.json`, `playwright.config.ts`, `tests/browser`, `tests/fixtures`, `static/`, and `svelte.config.js`. Verify the repo has no `.svelte` or `.ts` files outside `node_modules` and `mise run release --dry-run` still works.
- [ ] 6.3 Update `hk.pkl` (drop `svelte-check`) and `ci.yml` (drop Node, npm, and the binding diff check). Disable `release.yml` until `native-release-pipeline`. Verify `hk check` passes and CI is green on both platforms.

## 7. Layout

- [ ] 7.1 `git mv` the crate to the repo root: `Cargo.toml`, `src/`, `ui/`, `build.rs`, `icons/`, and the plists. Fix paths in CI, `hk.pkl`, and docs. Verify `cargo test`, `hk check`, and CI pass from the root.

## 8. Docs and tooling cleanup

- [ ] 8.1 Rewrite `AGENTS.md` for the new stack: Slint and std-widgets, the icon registry rule, renderer rules, the debug-only Slint MCP command, and any traps found in groups 4 and 5. Drop the Tauri, Svelte, Bits UI, asset protocol, `tauri-action`, and fixture server facts. Verify `rg -i "tauri|svelte|bits ui|webview" AGENTS.md` only finds history notes, if any.
- [ ] 8.2 Add the Slint attribution badge to `README.md` and update its build instructions. Verify the README build steps work from a fresh clone.
- [ ] 8.3 Remove the 51 `tauri*` skills and `svelte-code-writer` from `.claude/skills` and `skills-lock.json`, and set `"svelte@svelte": false` in `.claude/settings.json`. Verify a new Claude Code session in the repo lists no Tauri or Svelte skills.

## 9. Manual checks on the real OS

- [ ] 9.1 Windows release build: activation and held activation focus the prompt, arrowing scrolls, Ctrl+K and Escape stages, paste reports a failure on next activation, tray menu works, autostart entry applies, a second launch shows the first, Japanese IME types into the prompt, Task Manager shows one Dango process. Record the idle private working set in the PR or commit body.
- [ ] 9.2 macOS release build: the same checks with Cmd+K, over a fullscreen app, with Accessibility and Input Monitoring granted, and Activity Monitor shows one Dango process.
