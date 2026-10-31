# Dango

A keyboard-first launcher and command palette for Windows and macOS, in the
spirit of Raycast. A tray app that stays resident, listens on one global
shortcut, and summons a command prompt over whatever the user is doing.

Personal hobby project. No external users. Optimise for the author's daily use
and for a codebase that stays pleasant to work in. Do not add process,
telemetry, or infrastructure that only pays off at a scale this project will
never reach.

## Sources of truth

- `openspec/specs/` - what the app does. Every behaviour rule lives here.
- `AGENTS.md` (this file) - how to work on the code: stack, architecture,
  conventions, platform traps, build facts.
- `openspec/config.yaml` - rules for writing OpenSpec artifacts.
- GitHub issues - open work and checks not yet run.

Do not duplicate a rule across these. If a spec already says it, link to the
spec.

## Workflow

- A change to behaviour goes through OpenSpec: `/opsx:propose`, then
  `/opsx:apply`, then `/opsx:archive`. Use `/opsx:explore` to think first.
- A fix that does not change specified behaviour can be done directly.
- Agents commit their own work as they go, without asking: atomic
  conventional commits, each one building and passing its tests. Pushing
  still waits for the author.
- Run `hk check` before pushing: `cargo fmt` and `cargo clippy -D warnings`,
  which also compiles the `.slint` files. Tests, UI included: `cargo test`.
- Work one platform at a time. Finish and verify on the machine at hand, and
  leave the other platform's hand checks open until a session on that
  machine. Do not switch back and forth within a session.
- A check that cannot be run on one platform becomes a GitHub issue, not a
  note in an archived change.
- When you hit a platform trap or build fact not listed below, add it here in
  the same change.

## Project stage

Alpha with no users. Breaking changes to the config file, the extension
manifest, the view protocol, and stored data are free: no migrations, no
compatibility shims, no deprecation paths. Keep the version fields, but do not
spend design on honouring old values.

## Platforms

- Windows and macOS are both first class. Every feature ships on both, or the
  spec states the gap and why.
- Linux is out of scope. No Linux code, no Wayland design, no Linux branches
  "just in case".
- Development happens on both macOS and Windows, daily use on Windows.
- Text input covers English and German keyboards only. No IME, non-Latin
  script, or right-to-left requirements, checks, or tests.
- OS-specific code sits behind narrow Rust traits (for example `WindowManager`,
  `ClipboardSource`, `KeyMonitor`, `AppIndexer`), implemented in
  `platform/windows` and `platform/macos`. Never scatter `cfg` attributes
  through feature code.

## Tech stack

- One Rust crate at the repo root: all OS integration, all business logic,
  and the UI. Rust in `src/`, `.slint` files in `ui/`.
- Slint draws the launcher and the tray in the Dango process, on the winit
  backend with the Fluent style on both platforms. Build on `std-widgets.slint`
  first. The UI is a renderer of the launcher snapshot, not a place for logic.
- One palette, the `Palette` global in `ui/app.slint`. Every colour, radius,
  and type size comes from it; Rust passes indexes into it, never colours.
- Named icons come from `icondata_lu` behind one registry in
  `src/slint_shell/icons.rs`. Never draw an icon elsewhere and never hand-roll
  an SVG: add the name to that registry. An extension names one as
  `icon:<name>`; a value without that prefix is a path to an image file.
- SQLite for local persistence.
- Node, the OpenSpec CLI, hk, git-cliff, and cargo-release are pinned via mise
  (`mise.toml`). Node runs only the OpenSpec CLI and `scripts/*.mjs`.

### Off the shelf before hand-rolled

A hard requirement. Before writing code that does a general job, look for a
maintained crate or package and pick it unless it genuinely cannot do the job.
Hand-roll only when nothing suitable exists or the candidate provably falls
short, and then the design names the candidates and what they could not do.
"I did not look" is not a reason, and neither is a guess: check the API before
ruling a library out.

## Architecture

Everything the user can invoke is a command, and every command belongs to an
extension. Built-in features are extensions too. The contract is in
`openspec/specs/extension-model` and `openspec/specs/view-protocol`.

- Built-ins run as native Rust. Third-party extensions will later run in a
  sandboxed host. Do not route built-ins through the sandbox.
- The manifest and the view protocol are versioned contracts. A change that
  touches them says so in its design.
- The launcher's interaction state lives in one Rust state machine,
  `src/ui/`: query, selection, the view stack, the action panel,
  Escape stages, and what each key does. It is a pure reducer, tested with
  `cargo test` and no window. No interaction rule lives in the renderer.
- `src/app/` holds `Core`: the state machine, the hold, the services, the
  hotkeys, and the tray status. Window and tray work goes through the narrow
  `Shell` trait (`src/shell.rs`), which `src/slint_shell/` implements and a
  recording fake implements in tests.
- The renderer (`src/slint_shell/render.rs`) turns each snapshot into Slint
  models and properties, replacing everything. It sends intents to
  `Core::dispatch` in order on the main thread. A root `FocusScope` asks Rust
  through `capture-key-pressed` whether a chord is in the snapshot's
  `bindings` before the focused control sees it. The renderer keeps only
  hover, focus placement, and scroll-into-view.
- Slint owns the main thread; a tokio runtime owned by the shell runs beside
  it. Backend work reaches the UI through `Shell::on_main_thread`, which is
  `slint::invoke_from_event_loop`.
- Secrets live only in `auth.json` (see `openspec/specs/ai-providers`). The read
  sits behind one function, so the `keyring` crate can take it over later.

## Performance

Keep the UI snappy. Speed comes from structure, not from timing numbers:

- The launcher component is built at startup and only shown and hidden after.
  winit creates its native window on the first show, which costs that first
  activation a little more.
- Search never blocks. Providers are async, slow ones appear late, and each
  keystroke cancels the previous search instead of debouncing.
- View updates replace the full tree. No reconciler, no diffing.
- Defer work away from startup and the hotkey path when it can run lazily.

Do not put millisecond budgets in specs or tests. Results depend on machine
load and are not reliable.

## Conventions

- Conventional commits. Imperative subject under 50 characters, no trailing
  period. Atomic commits.
- Be extremely conservative with code comments. Comment only non-obvious
  reasoning, a surprising constraint, or a subtle workaround.
- Rust: borrow over clone, `Result` with typed errors over strings, small
  focused modules.
- UI is built on `std-widgets.slint` with markup close to the documented
  example. Hand-roll a surface only where no widget fits, and say why in the
  design. Interactive elements carry an `accessible-label`, which is also how
  the UI tests find them.
- UI tests run the real state machine against the component on
  `i_slint_backend_testing` (`src/slint_shell/tests.rs`).
- Any element that needs an icon uses one. A placeholder square only where the
  icon is genuinely unknown.
- User-facing text is written with the `ux-writing` skill and against
  `openspec/specs/interface-copy`.

## Platform traps

- Windows app indexing must enumerate `shell:AppsFolder`, not Start Menu `.lnk`
  files. Only that route sees UWP and MSIX apps, and it gives AppUserModelIDs
  for launching plus icons via `IShellItemImageFactory::GetImage`.
- A `shell:AppsFolder` target can be a `::{GUID}` shell namespace identifier
  (File Explorer is one). Its second character is a colon, so a naive
  drive-letter test takes it for a path. Classify paths with
  `Path::components` and `Component::Prefix`.
- Windows window management must correct geometry with
  `DWMWA_EXTENDED_FRAME_BOUNDS`, or invisible resize borders leave gaps.
  Multi-monitor tiling needs per-monitor DPI awareness.
- Restoring focus to the previous foreground window on Windows hits the
  foreground lock and needs `AttachThreadInput` and `AllowSetForegroundWindow`.
  Paste, snippet expansion, and AI replace-selection all depend on it.
  `AttachThreadInput` is in `Win32::System::Threading`, not
  `Win32::UI::WindowsAndMessaging`.
- A non-elevated process cannot hook or manage elevated windows on Windows.
  Accept the limit.
- On Windows, `WS_EX_TOOLWINDOW` is re-applied after each show: tao rewrote
  the extended style on every visibility change, and nobody has checked
  whether winit does. Windows positions in physical pixels, macOS in logical
  points. Do not share the positioning code.
- macOS gates window management, paste, snippet expansion, and reading the
  selection behind Accessibility.
- enigo's `Enigo::new` on macOS fails with `NoPermission` without
  Accessibility. Create it on first use, never at startup.
- Keyboard `CGEventTap`s on macOS, listen-only and active, need Input
  Monitoring, not Accessibility. An Input Monitoring grant applies only after
  a restart; an Accessibility grant applies at once.
- Showing the launcher over fullscreen apps on macOS needs an NSPanel.
- Reading the selection is direct on macOS via `AXSelectedText`. On Windows the
  realistic path is copy, read clipboard, restore clipboard.
- An ad-hoc signed macOS build has a designated requirement of its own
  `cdhash`, which changes every build, so macOS can treat each update as a new
  app and drop its Accessibility and Input Monitoring grants. Release builds
  are signed with one certificate from CI secrets to keep the requirement
  stable.
- A real signature turns on the hardened runtime. Under it, `osascript` talking
  to Finder (Empty Trash) needs the `com.apple.security.automation.apple-events`
  entitlement in `Entitlements.plist` and an `NSAppleEventsUsageDescription`
  in `Info.plist`, or tccd denies it.
- winit never reports focus for the Slint launcher on Windows: the window is
  the foreground and has the keyboard, yet `has_focus` stays false and no
  `Focused` event arrives. Focus loss is watched with a
  `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)` in `platform/windows` instead.
- Slint's winit backend probes for OpenGL 2 on Windows and silently falls back
  to the software renderer without it. That renderer refuses
  `set_rendering_notifier`, so paint timing hangs off `RedrawRequested` in the
  window event filter.

## Build and verification facts

- Slint's `ElementHandle` test API finds nothing without debug info in the
  generated code, so `build.rs` turns it on for the debug profile.
- The Slint tray links Common Controls 6. A Windows binary without the
  manifest that selects it fails to load (`STATUS_ENTRYPOINT_NOT_FOUND`), the
  test binary included, so `build.rs` has the linker embed it in every
  artifact.
- A Slint `ListView` estimates its height from the rows it has built and
  clamps a scroll to that estimate, so scrolling to an unbuilt row stops
  short. The result list builds every row in a `ScrollView`; it holds at most
  50.
- Windows cannot be cross-compiled from macOS here. CI on `windows-latest` is
  the only Windows compile check from the Mac. Check Windows symbol names and
  module paths against the vendored crate source under `~/.cargo/registry`
  before pushing; macOS code the same way from Windows.
- Tauri used to switch on crate features for us. Without it, a missing
  feature shows up as an item "configured out" (`windows`, `time`): add the
  feature the compiler names.
- CI runs `cargo clippy -- -D warnings`. Two lints that bite in platform code:
  an inner `unsafe` block inside an `unsafe fn` (`unused_unsafe`) and casting a
  value to its own type (`unnecessary_cast`, easy with `HWND`).
- On the Mac the shell is fish: `VAR=value cmd` is invalid. Use
  `env VAR=value cmd`.
- A release build applies `launcher.autostart`, so running one from `target/`
  with the author's config points the login entry at that build. Test release
  builds with `DANGO_CONFIG_DIR` set to a scratch directory.
- `DANGO_MEASURE=1` enables activation timing logs (stderr) in a release build.
  A `windows_subsystem = "windows"` build shows stderr only when it is
  redirected.
- Release sets `panic = "abort"`, and a release build on Windows has no
  console, so a panic there is a silent exit.
- The Slint MCP server is for debug builds only, and never enabled in
  `Cargo.toml`, so a release cannot open the port:
  `env SLINT_MCP_PORT=9315 cargo run --features slint/mcp`. It starts
  listening on the first window show; `.mcp.json` points `dango-ui` at it.
- The updater and the release workflow are off until
  `native-release-pipeline`. Builds are installed by hand.
- A macOS bundle without `APPLE_SIGNING_IDENTITY` in the environment is
  ad-hoc signed and carries no entitlements. Set it to the identity from
  `security find-identity -v -p codesigning` to test a signed build.
