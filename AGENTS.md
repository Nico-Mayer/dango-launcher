# Dango

Keyboard-first launcher + command palette for Windows and macOS, Raycast-style. Tray app stays resident, listens on one global shortcut, summons command prompt over whatever user doing.

Personal hobby project. No external users. Optimise for author's daily use + codebase pleasant to work in. No process, telemetry, or infra that only pays off at scale project never reaches.

## Sources of truth

- `openspec/specs/` - what app does. Every behaviour rule lives here.
- `AGENTS.md` (this file) - how to work on code: stack, architecture, conventions, platform traps, build facts.
- `openspec/config.yaml` - rules for writing OpenSpec artifacts.
- GitHub issues - open work.

No rule duplicated across these. If spec says it, link to spec.

## Workflow

- Behaviour change goes through OpenSpec: `/opsx:propose`, then `/opsx:apply`, then `/opsx:archive`. `/opsx:explore` to think first.
- Fix not changing specified behaviour: do directly.
- Agents commit own work as they go, no asking: atomic conventional commits, each builds + passes tests. Pushing still waits for author.
- Run `mise run check` before push: `cargo fmt` and `cargo clippy --all-targets -- -D warnings` on every file, also compiles `.slint` files. Tests, UI included: `cargo test`. Plain `hk check` only checks changed files, so clean tree = checks nothing.
- Repeated commands = mise tasks (`mise tasks` lists). They set own env vars + pick platform's command, so same in fish, PowerShell, bash.
- One platform at a time. Finish + verify on machine at hand; leave other platform's hand checks as open tasks in change's `tasks.md` until session on that machine. No archiving change with open hand checks. No switching back and forth within session.
- Hit platform trap or build fact not listed below → add here in same change.

## Project stage

Alpha, no users. Breaking changes to config file, extension manifest, view protocol, stored data are free: no migrations, no compat shims, no deprecation paths. Keep version fields, but no design spent honouring old values.

## Platforms

- Windows + macOS both first class. Every feature ships on both, or spec states gap + why.
- Linux out of scope. No Linux code, no Wayland design, no Linux branches "just in case".
- Dev on macOS + Windows, daily use on Windows.
- Text input: English + German keyboards only. No IME, non-Latin script, or RTL requirements, checks, tests.
- OS-specific code behind narrow Rust traits (e.g. `WindowManager`, `ClipboardSource`, `KeyMonitor`, `AppIndexer`), implemented in `platform/windows` and `platform/macos`. Never scatter `cfg` attributes through feature code.

## Tech stack

- One Rust crate at repo root: all OS integration, business logic, UI. Rust in `src/`, `.slint` files in `ui/`.
- Slint draws launcher + tray in Dango process, winit backend. UI = renderer of launcher snapshot, no logic. Look = design system in `openspec/specs/design-system`.
- One `Theme` global in `ui/theme.slint`. Every colour, radius, size, typeface from it. Colours use herdr role names, values from private `ThemeColors` preset: Graphite when dark, Paper when light. New theme = one more preset value, no component change. Reads OS scheme via std-widgets' `Palette.color-scheme`, only reason build keeps std-widgets style. Rust never passes colour.
- `ui/` holds `theme.slint`, `atoms/` (icon, kbd, tag, accessory, dot, form controls), `shell/` (window, SearchBar, ActionBar, action panel), `views/` (list, grid, detail, form, empty view), and `app.slint`, exporting window, tray, globals. Each part maps to one design part.
- Controls our own, built on `TextInput`, `FocusScope`, `TouchArea`, `PopupWindow`. std-widgets controls can't take our colours, radii, focus border (palette is `out`), so only `ScrollView` used from them. Control carries `accessible-role`, `-label`, `-value`, `-description` itself, inner `TextInput` set to `accessible-role:
  none`.
- Named icons from `icondata_lu` behind one registry in `src/slint_shell/icons.rs`. Never draw icon elsewhere, never hand-roll SVG: add name to registry. Extension names one as `icon:<name>`; value without prefix = image file path.
- SQLite for local persistence.
- Node, hk, git-cliff, cargo-release, cargo-packager, slint-lsp, skillshare pinned via mise (`mise.toml`). OpenSpec CLI from mise too, on `latest`. Node runs only OpenSpec CLI and `scripts/*.mjs`.
- Agent setup (MCP servers, plugins, own skills) lives in `.skillshare/config.yaml`, managed with skillshare CLI in project mode (`-p`). Change it through `skillshare mcp`, `skillshare plugin`, `skillshare install`, never by editing `.mcp.json` or Claude plugin settings. `mise run agents` applies it on a machine, also after clone. Tool-owned skills stay with their tool: OpenSpec skills + `/opsx` commands from `openspec update`, hk skills linked by mise (`mise skills sync`).
- Rust pinned in `rust-toolchain.toml`. Global mise `rust` entry exports `RUSTUP_TOOLCHAIN`, which beats that file, so `mise.toml` tells mise to read it. Bump channel there, nowhere else.

### Off the shelf before hand-rolled

Hard requirement. Before writing code for general job, look for maintained crate/package, pick it unless it genuinely can't do job. Hand-roll only when nothing suitable exists or candidate provably falls short; then design names candidates + what they couldn't do. "I did not look" not a reason, nor a guess: check API before ruling library out.

## Architecture

Everything user can invoke = command; every command belongs to extension. Built-ins are extensions too. Contract in `openspec/specs/extension-model` and `openspec/specs/view-protocol`.

- Built-ins run native Rust. Third-party extensions later run in sandboxed host. Don't route built-ins through sandbox.
- Manifest + view protocol = versioned contracts. Change touching them says so in design.
- Launcher interaction state lives in one Rust state machine, `src/ui/`: query, selection, view stack, action panel, Escape stages, what each key does. Pure reducer, tested with `cargo test`, no window. No interaction rule in renderer.
- `src/app/` holds `Core`: state machine, hold, services, hotkeys, tray status. Window + tray work goes through narrow `Shell` trait (`src/shell.rs`), implemented by `src/slint_shell/` and by recording fake in tests.
- Renderer (`src/slint_shell/render/`) turns each snapshot into Slint globals, replacing everything, one writer per global: `Shell`, `Search`, `ListData`, `GridData`, `EmptyData`, `DetailData`, `Panel`, `FormData`. Components read globals they need, nothing threaded through window. Callbacks go through `Intents` global to `Core::dispatch`, in order on main thread. Root `FocusScope` asks Rust via `capture-key-pressed` whether chord is in snapshot's `bindings` before focused control sees it. Renderer keeps only hover, focus placement, scroll-into-view.
- Rows, headings, tiles, panel entries have fixed heights, so renderer places, reveals, sizes window by counting. Change height in `Theme` and matching constant in `render/` together.
- Slint owns main thread; tokio runtime owned by shell runs beside it. Backend work reaches UI via `Shell::on_main_thread`, which is `slint::invoke_from_event_loop`.
- Secrets only in `auth.toml` (see `openspec/specs/ai-providers`). Read behind one function, so `keyring` crate can take over later.

## Performance

Keep UI snappy. Speed from structure, not timing numbers:

- Launcher component built at startup, only shown/hidden after.
- Search never blocks. Providers async, slow ones appear late, each keystroke cancels previous search instead of debouncing.
- View updates replace full tree. No reconciler, no diffing.
- Defer work away from startup + hotkey path when lazy possible.

No millisecond budgets in specs or tests. Results depend on machine load, unreliable.

## Conventions

- Conventional commits. Imperative subject under 50 chars, no trailing period. Atomic commits.
- Extremely conservative with code comments. Comment only non-obvious reasoning, surprising constraint, subtle workaround.
- Rust: borrow over clone, `Result` with typed errors over strings, small focused modules.
- Surface built from design system parts in `ui/atoms/`, `ui/shell/`, `ui/views/`. New part goes where design puts it, takes every value from `Theme`. Interactive elements carry `accessible-label`, also how UI tests find them.
- UI tests run real state machine against component on `i_slint_backend_testing` (`src/slint_shell/tests.rs`).
- Any element needing icon uses one. Placeholder square only where icon genuinely unknown.
- User-facing text written with `ux-writing` skill and against `openspec/specs/interface-copy`.

## Platform traps

- Windows app indexing must enumerate `shell:AppsFolder`, not Start Menu `.lnk` files. Only that route sees UWP + MSIX apps, gives AppUserModelIDs for launching plus icons via `IShellItemImageFactory::GetImage`.
- `shell:AppsFolder` target can be `::{GUID}` shell namespace identifier (File Explorer is one). Second char is colon, so naive drive-letter test takes it for path. Classify paths with `Path::components` and `Component::Prefix`.
- Windows window management must correct geometry with `DWMWA_EXTENDED_FRAME_BOUNDS`, or invisible resize borders leave gaps. Multi-monitor tiling needs per-monitor DPI awareness.
- Restoring focus to previous foreground window on Windows hits foreground lock, needs `AttachThreadInput` and `AllowSetForegroundWindow`. Paste, snippet expansion, AI replace-selection all depend on it. `AttachThreadInput` is in `Win32::System::Threading`, not `Win32::UI::WindowsAndMessaging`.
- `force_foreground` detaching input queues can leave launcher in front with no focus window (seen when shown over herdr-app). Windows then sends every key as `WM_SYSKEYDOWN`/`WM_SYSCHAR`; winit 0.30 can pass `WM_SYSCHAR` to `DefWindowProc`, which beeps on every key while text still types. Launcher `did_show` calls `SetFocus` after it. `SetFocus` works only from window's own thread, so selection read's lend restore (worker thread) can't use it.
- Windows reports AltGr as left Ctrl + right Alt, so chord check must look at `VK_RMENU` before treating Ctrl+Alt as shortcut, or every AltGr char (`@`, `{`, `\` on German layout) reads as one.
- `open::with` and `open::with_detached` on Windows look bare app name up under App Paths but can keep trailing NUL (5.4.5), so `Command` refuses path. Pass full exe path; browser opener reads it with `windows-registry` in `platform/windows/browser.rs`.
- Non-elevated process can't hook or manage elevated windows on Windows. Accept limit.
- Windows silently removes `WH_KEYBOARD_LL` hook whose procedure exceeds `LowLevelHooksTimeout`, without warning. `platform/windows/llhook.rs` replaces hook on timer. Each replacement puts Dango first in hook chain, ahead of AutoHotkey or PowerToys even if they started later, so a remap there of a key Dango swallows (CapsLock) stops working.
- New Notepad (and likely other WinUI apps) can handle injected key late: Ctrl released right after V reads as plain `v`. Synthetic chords hold modifier briefly after key (`CHORD_HOLD`).
- On Windows, `WS_EX_TOOLWINDOW` re-applied after each show: winit rewrites extended style from own flags on every visibility change (`apply_diff` in `window_state.rs`). Windows positions in physical pixels, macOS in logical points. Don't share positioning code.
- winit `with_active(false)` holds for every show, not only first (`apply_diff` works on copy of flags). Notice window gets it from `with_winit_window_attributes_hook`, which runs when Slint makes window adapter, before component title set: hook can't tell windows apart by title, so `CREATING_NOTICE` flag set around notice creation.
- winit `set_cursor_hittest(false)` on Windows makes window `WS_EX_LAYERED | WS_EX_TRANSPARENT` but never calls `SetLayeredWindowAttributes`. Layered window without it draws nothing, so notice window's `did_show` sets full alpha.
- macOS gates window management, paste, snippet expansion, reading selection behind Accessibility.
- Never query system-wide AX element (`AXUIElement::new_system_wide`) from Dango while panel key: macOS answers in-process on calling thread, accesskit panics on `RefCell` main thread holds, Dango aborts. Ask frontmost app's element (pid from `NSWorkspace`) instead.
- enigo's `Enigo::new` on macOS fails with `NoPermission` without Accessibility. Create on first use, never at startup.
- Keyboard `CGEventTap`s on macOS need Input Monitoring to see events; active one (hyperkey) also needs Accessibility, or creation is refused. Input Monitoring grant applies only after restart; Accessibility grant applies at once.
- Revoking Accessibility under running active tap stalls all keyboard + mouse input system-wide. Permission poller restarts hyperkey on any permission change, so new tap matches current grants.
- Showing launcher over fullscreen apps on macOS needs NSPanel.
- Launcher window on macOS takes mouse clicks on its fully transparent pixels too (gap, area below card): app behind gets none, launcher's own `TouchArea` does.
- Mouse click on launcher panel activates Dango on macOS despite `NonactivatingPanel` mask (hotkey show does not). macOS `restore_previous_focus` is a no-op, so after a click + hide Dango stays active app, focus not back to app behind.
- Notice window on macOS reclassed to plain `NSPanel`, never launcher's `DangoPanel`: that class answers yes to `canBecomeKeyWindow` and writes global `PANEL_IS_KEY` in `becomeKeyWindow`. Borderless `NSPanel` never becomes key.
- winit resizes macOS window with `setContentSize`, keeping bottom-left origin, so Slint `Window::set_size` or changed `height` grows launcher upward + moves SearchBar. Height goes through `LauncherWindow::set_height`, which keeps top edge, only where `keeps_top_edge` true. Set component's `height` before native resize: Slint sizes window back to height its component allows, within same `SetWindowPos`. Windows uses `SetWindowPos` with `SWP_NOMOVE`; macOS has no implementation yet (`setFrame:display:` with origin moved down), stays full height.
- Reading selection direct on macOS via `AXSelectedText`. On Windows realistic path: copy, read clipboard, restore clipboard.
- Ad-hoc signed macOS build has designated requirement of own `cdhash`, changes every build, so macOS may treat each update as new app, drop Accessibility + Input Monitoring grants. Release builds signed with one certificate from CI secrets to keep requirement stable.
- Real signature turns on hardened runtime. Under it, `osascript` talking to Finder (Empty Trash) needs `com.apple.security.automation.apple-events` entitlement in `Entitlements.plist` and `NSAppleEventsUsageDescription` in `Info.plist`, or tccd denies.
- winit never reports focus gain for Slint launcher on Windows: window is foreground + has keyboard, yet `has_focus` stays false. Focus loss watched with `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)` in `platform/windows` instead. winit still sends `Focused(false)` when another window takes foreground, selection read's lend included, so that handler checks `platform::focus_lent` too.
- Command hotkey's `Core::show` is queued on main thread. Command run queued behind it, or selection read lends foreground before launcher holds it and times out.
- On macOS launcher's native window exists before first show, so missing window handle ≠ "first show". One-time `attach` (panel reclass, non-activating mask, level, Spaces) runs from flag in `SlintShell`. Without it launcher never gets keyboard, typing goes to app behind.
- Slint's winit backend probes for OpenGL 2 on Windows, silently falls back to software renderer without it. That renderer refuses `set_rendering_notifier`, so paint timing hangs off `RedrawRequested` in window event filter.
- Tray app's PATH lacks what shell adds. Windows: Explorer gives saved PATH, without `mise activate` folders (author's saved PATH has no mise folder). macOS: launchd PATH only, no shell profile. Agent CLI lookup adds known install folders + (macOS) login shell's PATH, and passes same list as child's PATH so npm `.cmd` wrappers find `node`.
- Child process of GUI-subsystem Dango on Windows gets visible console unless `CREATE_NO_WINDOW`, which takes focus and hides launcher. With process-wrap's `JobObject`, set flag through `CreationFlags` wrapper, never on inner `Command`: `JobObject` overwrites inner command's flags. Dev and test runs have console, so only release build shows it.
- Held global hotkey never repeats: `global-hotkey` registers with `MOD_NOREPEAT` on Windows, Carbon sends one press on macOS. Both report release (Windows: crate polls `GetAsyncKeyState` every 50 ms). Command with `repeats` in manifest re-run by `Core` until release.
- `ddc-macos` records pause DDC/CI needs after each command but never waits for it; only `DdcHost::sleep()` does. Shared adapter in `platform/brightness.rs` sleeps after every read + write while bus lock held. dxva2 on Windows times itself; there `sleep()` is no-op.

## Build and verification facts

- hk pre-commit hook stashes unstaged files, dropping `MERGE_HEAD`, so `git commit` after conflicted merge fails with "could not open `.git/MERGE_HEAD`". Staged tree survives: commit with `git commit-tree "$(git write-tree)" -p main -p <branch>` and fast-forward to result.
- Software renderer clips children to square, ignores clipping element's `border-radius`. Child filling rounded corner, e.g. ActionBar's background, rounds that corner itself.
- Slint's `ElementHandle` test API finds nothing without debug info in generated code, so `build.rs` turns it on for debug profile.
- Slint tray links Common Controls 6. Windows binary without manifest selecting it fails to load (`STATUS_ENTRYPOINT_NOT_FOUND`), test binary included, so `build.rs` has linker embed it in every artifact.
- Slint `ListView` estimates height from rows built, clamps scroll to estimate, so scrolling to unbuilt row stops short. Every list, root and pushed, builds every row in `ScrollView`; holds max 50 items (`ui::ITEM_LIMIT`).
- Windows can't be cross-compiled from macOS here. CI on `windows-latest` = only Windows compile check from Mac. Check Windows symbol names + module paths against vendored crate source under `~/.cargo/registry` before push; macOS code same way from Windows.
- Tauri used to switch on crate features for us. Without it, missing feature shows as item "configured out" (`windows`, `time`): add feature compiler names.
- CI runs `cargo clippy --all-targets -- -D warnings`, linting tests too. Two lints that bite in platform code: inner `unsafe` block inside `unsafe fn` (`unused_unsafe`) and casting value to own type (`unnecessary_cast`, easy with `HWND`).
- `mise run` installs every missing tool in `mise.toml` before task runs, cargo-packager compiled from source included. CI sets `MISE_TASK_RUN_AUTO_INSTALL=false`, installs what it needs via mise-action's `install_args`. CI task needing new tool fails until tool added there.
- Cache saved by tag run readable only by same tag, so release runs save no cache, neither Rust's nor mise's. Would only fill 10 GB cache limit.
- On Mac shell is fish: `VAR=value cmd` invalid. Use `env VAR=value cmd`.
- Release build applies `launcher.autostart`, so running one from `target/` with author's config points login entry at that build. Test release builds with `DANGO_CONFIG_DIR` set to scratch dir.
- `DANGO_CONFIG_DIR` moves only config file. Database (data dir) + clipboard images (cache dir) stay in author's real `com.nimayer.dango` folders, so hand check that copies things lands in real clipboard history.
- `DANGO_MEASURE=1` enables activation timing logs (stderr) in release build. `windows_subsystem = "windows"` build shows stderr only when redirected.
- Release sets `panic = "abort"`, release build on Windows has no console, so panic there = silent exit.
- `toml` and `toml_edit` error text quotes the source line it failed on. Secrets file (`auth.toml`) reports only the line, from error's span through `config::line_of`, or key lands in log + tray.
- User files in config dir follow `openspec/specs/config-files`. New one gets `UserFile` variant in `src/config/files.rs`: path, `#:schema` header, schema, reference section all come from there.
- Slint MCP server debug builds only, never enabled in `Cargo.toml`, so release can't open port: `mise run ui`, which is `cargo run --features slint/mcp` with `SLINT_MCP_PORT=9315`. Starts listening on first window show.
- `dango-ui` in `.mcp.json` = stdio proxy in `tools/dango-ui-mcp`, own crate + target folder, so always connected. Always lists own tools: `launch_dango` (runs `cargo build --features slint/mcp`, starts `target/debug/dango` with `SLINT_MCP_PORT=9315`, shows launcher once, waits), `restart_dango` (kills this checkout's debug build by exe path, never installed Dango, then launches; use after code change), `press`, `snapshot`. Slint's tools appear once build listens, also when started by hand. Build + app output go to `target/dango-ui-mcp.log`. `launch_dango` fails while another Dango runs. First session after clean checkout compiles proxy; if Claude Code times out, build once with `cargo build --manifest-path tools/dango-ui-mcp/Cargo.toml`. Proxy builds with pinned toolchain, so same after toolchain bump.
- Windows can't replace running exe, so proxy open in one session made `cargo run` in next fail to rebuild it (`os error 5`, server shows CONNECTION_CLOSED). At start proxy renames own exe (both names: cargo hard-links it from `deps/`) and copies it back; copy keeps timestamp, so cargo still sees build fresh.
- Drive launcher through `dango-ui`, not SendInput: `press` for keys (`down`, `ctrl+k`, `escape`, `text:fire`), `snapshot` for compact list of labelled + interactive elements with handles, `take_screenshot` for window pixels. Slint's `dispatch_key_event` takes Slint's raw key chars; arrows + modifiers are control/private-use chars that never arrive from agent, so `press` maps names to them. `set_element_value` leaves caret at start: `press` `end` before typing more.
- `mise run keys` + `mise run shot` (wrap `scripts/*.ps1`, Windows only) only for what lives outside launcher window: global hotkey (`mise run keys alt+space`), focus + paste into other apps, placement on screen, multiple monitors. `shot` captures every display, prints PNG path. `down:` held through SendInput doesn't auto-repeat: send key many times to mimic holding. Keys go to whatever in front, so check foreground window is Dango before sending Enter, Escape, Ctrl chords. Launcher hotkey sent through `keys` can stop working for a while: Dango shows, `SetForegroundWindow` is refused, next foreground event hides it again. Hand presses work at the same moment, so treat as test-setup limit, not bug. Check foreground after every hotkey press, stop on mismatch.
- On macOS no `keys`/`shot` task yet. `osascript` System Events `keystroke ... using command down` sends no modifier key-down, so winit sees Cmd+Enter as plain Enter, Cmd+A as `a`: post chords as CGEvents (modifier down, key, modifier up). Global hotkeys still fire from `osascript`. Showing launcher never activates Dango, so `activate` of app behind changes nothing; leaving launcher needs a real CGEvent mouse click. `screencapture` needs Screen Recording for terminal: without it, take window pixels with Slint's `take_screenshot`, also callable as plain JSON-RPC POST to `http://127.0.0.1:9315/mcp` when session lacks Slint's tools. System Events lists no notice window: check on-screen windows with `CGWindowListCopyWindowInfo`. Locked screen refuses all of it.
- `slint-lsp` from mise `github:` backend, kept at `slint` crate version. Slint plugin's LSP runs `slint-lsp` from PATH.
- skillshare installs Claude plugin through local marketplace and writes that machine's path into `.claude/settings.json`, so file is untracked. Plugin intent lives in `.skillshare/config.yaml`.
- Packaging is `cargo-packager`, configured in `[package.metadata.packager]` in `Cargo.toml` with camelCase keys. `mise run package` builds `nsis` on Windows and `app,dmg` on macOS. With `CARGO_PACKAGER_SIGN_PRIVATE_KEY` set, writes minisign `.sig` next to each output and tars `.app` into `.app.tar.gz` updater installs.
- cargo-packager writes no update manifest. `manifest` job in `release.yml` builds `latest.json` from uploaded `.sig` files.
- cargo-packager reads macOS signing identity only from its config, and `--config` replaces `Cargo.toml` table instead of merging. CI writes `signingIdentity` into `Cargo.toml` before packaging. Without it bundle not codesigned, no entitlements. To test signed build, add `signingIdentity` locally with identity from `security find-identity -v -p codesigning`, don't commit.
- Local macOS bundle sharing `com.nimayer.dango` with installed Dango gets confused with it in System Settings' permission lists, and macOS's reopen after Input Monitoring grant starts installed one. Hand check permissions on copy with own `CFBundleIdentifier`, ad-hoc signed (`codesign --force --sign - --entitlements Entitlements.plist`), placed outside `/tmp` (grants there don't stick). Start it with `open --stderr <file> --env DANGO_CONFIG_DIR=<dir>`: shares instance lock with installed Dango, so quit that first. `open` hands app caller's environment, shell PATH included; for what Finder start gets, run it as `env -i HOME=$HOME USER=$USER SHELL=<login shell> PATH=/usr/bin:/bin:/usr/sbin:/sbin open ...`. macOS's reopen after Input Monitoring or Accessibility grant drops `--env`, so reopened copy runs on author's real config and its autostart points login entry at copy: start it again by hand, and start installed Dango after test to restore login entry.
- `single-instance` on macOS treats name as file path, so bare name lands in working dir: `/` and read-only for app opened from Finder or at login, where Dango then exits silently. Lock lives in data dir (`platform::instance_lock`).
- Opening running app from Finder or Dock starts no second process on macOS: running one gets reopen Apple event (`aevt`/`rapp`), handled by `platform::watch_reopen`. Only shell launch reaches single instance socket.
- On macOS updater swaps bundle in place and returns. Dango then releases single instance lock, restarts bundle with `open -n`. On Windows NSIS installer exits Dango and restarts it.