## 1. Spike: macOS panel and latency floor

- [ ] 1.1 Convert the Tauri window to an NSPanel with the non-activating style mask, verify it becomes key and receives typing while another application stays active
- [ ] 1.2 Verify the panel appears over a fullscreen application without switching spaces
- [ ] 1.3 Measure hotkey-to-paint on a release build on macOS and record the number
- [ ] 1.4 If the 80ms budget is not reachable, stop and revisit the design before continuing

## 2. Project stack

- [ ] 2.1 Replace SvelteKit with plain Svelte 5 and Vite, remove `@sveltejs/kit`, `adapter-static`, and `svelte.config.js`
- [ ] 2.2 Point `tauri.conf.json` `frontendDist` at the new Vite output and confirm `npm run tauri dev` still runs
- [ ] 2.3 Remove the `greet` command, the demo route, and the unused scaffold assets in `static/`
- [ ] 2.4 Set the application identifier, product name, and window defaults in `tauri.conf.json`

## 3. Platform boundary

- [ ] 3.1 Define the `LauncherWindow` trait with `show`, `hide`, `position_on_active_display`, and `restore_previous_focus`
- [ ] 3.2 Create `platform/macos` and `platform/windows` modules and the compile-time selection between them
- [ ] 3.3 Add a compile guard so a build for any target other than macOS or Windows fails with a clear message

## 4. Window behaviour on macOS

- [ ] 4.1 Set the activation policy to accessory so no dock icon appears
- [ ] 4.2 Apply the NSPanel conversion, collection behaviour for all spaces and fullscreen auxiliary, and always-on-top level
- [ ] 4.3 Implement show, hide, and positioning on the display holding the foreground window
- [ ] 4.4 Verify absence from Command+Tab whether the launcher is visible or hidden

## 5. Window behaviour on Windows

- [ ] 5.1 Apply the tool window extended style and topmost z-order, and confirm absence from the taskbar and Alt+Tab
- [ ] 5.2 Capture the foreground window before showing and implement focus restore on hide, handling foreground lock
- [ ] 5.3 Implement positioning on the display holding the foreground window, using per-monitor DPI aware work area geometry
- [ ] 5.4 Verify correct physical size and placement across two displays with different scaling factors

## 6. Tray and lifecycle

- [ ] 6.1 Add the tray icon with a menu offering toggle and quit
- [ ] 6.2 Create the launcher window hidden during startup and confirm the frontend finishes loading before first activation
- [ ] 6.3 Add single instance handling so a second launch shows the running instance and exits
- [ ] 6.4 Unregister the global shortcut and clean up the tray on quit

## 7. Global activation

- [ ] 7.1 Register the default shortcut, Option+Space on macOS and Alt+Space on Windows
- [ ] 7.2 Toggle visibility from the shortcut, including hiding when already visible
- [ ] 7.3 Handle registration failure by starting anyway, marking the tray menu, and keeping the launcher openable from the tray
- [ ] 7.4 Verify the shortcut still works after the machine wakes from sleep

## 8. Frontend placeholder

- [ ] 8.1 Build the empty prompt: a borderless rounded surface with a single text input and no results
- [ ] 8.2 Hide on Escape and on window blur
- [ ] 8.3 Handle the reset signal by clearing the input and returning to root state
- [ ] 8.4 Confirm the frontend loads exactly once across repeated show and hide cycles

## 9. Latency instrumentation

- [ ] 9.1 Timestamp the shortcut in the backend and send an activation id to the frontend
- [ ] 9.2 Report back from the frontend on the animation frame following render
- [ ] 9.3 Compute and log the elapsed time from the backend monotonic clock, in development builds only
- [ ] 9.4 Record release-build measurements for cold and warm activation on both platforms

## 10. CI

- [ ] 10.1 Add a GitHub Actions workflow building on `windows-latest` and `macos-latest`
- [ ] 10.2 Run `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test` on both
- [ ] 10.3 Run `svelte-check` and the frontend build
- [ ] 10.4 Confirm a deliberate Windows-only compile error fails the matrix

## 11. Verification

- [ ] 11.1 Walk every scenario in `specs/launcher-shell/spec.md` on macOS
- [ ] 11.2 Walk every scenario in `specs/launcher-shell/spec.md` on Windows
- [ ] 11.3 Walk every scenario in `specs/global-activation/spec.md` on both platforms
- [ ] 11.4 Confirm both cold and warm activation meet the 80ms budget on release builds on both platforms
