## 1. Spike: macOS panel and latency floor

- [x] 1.1 Convert the Tauri window to an NSPanel with the non-activating style mask, verify it becomes key and receives typing while another application stays active
- [x] 1.2 Verify the panel appears over a fullscreen application without switching spaces
- [x] 1.3 Measure hotkey-to-paint on a release build on macOS and record the number
- [x] 1.4 If the 80ms budget is not reachable, stop and revisit the design before continuing

## 2. Project stack

- [x] 2.1 Replace SvelteKit with plain Svelte 5 and Vite, remove `@sveltejs/kit`, `adapter-static`, and `svelte.config.js`
- [x] 2.2 Point `tauri.conf.json` `frontendDist` at the new Vite output and confirm `npm run tauri dev` still runs
- [x] 2.3 Remove the `greet` command, the demo route, and the unused scaffold assets in `static/`
- [x] 2.4 Set the application identifier, product name, and window defaults in `tauri.conf.json`

## 3. Platform boundary

- [x] 3.1 Define the `LauncherWindow` trait with `show`, `hide`, `position_on_active_display`, and `restore_previous_focus`
- [x] 3.2 Create `platform/macos` and `platform/windows` modules and the compile-time selection between them
- [x] 3.3 Add a compile guard so a build for any target other than macOS or Windows fails with a clear message

## 4. Window behaviour on macOS

- [x] 4.1 Set the activation policy to accessory so no dock icon appears
- [x] 4.2 Apply the NSPanel conversion, collection behaviour for all spaces and fullscreen auxiliary, and always-on-top level
- [x] 4.3 Implement show, hide, and positioning on the display holding the foreground window
- [x] 4.4 Verify absence from Command+Tab whether the launcher is visible or hidden

## 5. Window behaviour on Windows

- [x] 5.1 Apply the tool window extended style and topmost z-order, and confirm absence from the taskbar and Alt+Tab
- [x] 5.2 Capture the foreground window before showing and implement focus restore on hide, handling foreground lock
- [x] 5.3 Implement positioning on the display holding the foreground window, using per-monitor DPI aware work area geometry
- [ ] 5.4 Verify correct physical size and placement across two displays with different scaling factors

## 6. Tray and lifecycle

- [x] 6.1 Add the tray icon with a menu offering toggle and quit
- [x] 6.2 Create the launcher window hidden during startup and confirm the frontend finishes loading before first activation
- [x] 6.3 Add single instance handling so a second launch shows the running instance and exits
- [x] 6.4 Unregister the global shortcut and clean up the tray on quit

## 7. Global activation

- [x] 7.1 Register the default shortcut, Option+Space on macOS and Alt+Space on Windows
- [x] 7.2 Toggle visibility from the shortcut, including hiding when already visible
- [x] 7.3 Handle registration failure by starting anyway, marking the tray menu, and keeping the launcher openable from the tray
- [ ] 7.4 Verify the shortcut still works after the machine wakes from sleep

## 8. Frontend placeholder

- [x] 8.1 Build the empty prompt: a borderless rounded surface with a single text input and no results
- [x] 8.2 Hide on Escape and on window blur
- [x] 8.3 Handle the reset signal by clearing the input and returning to root state
- [ ] 8.4 Confirm the frontend loads exactly once across repeated show and hide cycles (`[dango] frontend mounted` must appear once in a session with many activations)

## 9. Latency instrumentation

- [x] 9.1 Timestamp the shortcut in the backend and send an activation id to the frontend
- [x] 9.2 Report back from the frontend on the animation frame following render
- [x] 9.3 Compute and log the elapsed time from the backend monotonic clock, in development builds only
- [x] 9.4 Record release-build measurements for cold and warm activation on macOS
- [ ] 9.5 Record release-build measurements for cold and warm activation on Windows

## 10. CI

- [x] 10.1 Add a GitHub Actions workflow building on `windows-latest` and `macos-latest`
- [x] 10.2 Run `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test` on both
- [x] 10.3 Run `svelte-check` and the frontend build
- [x] 10.4 Confirm a deliberate Windows-only compile error fails the matrix

## 11. Verification

- [x] 11.1 Walk every scenario in `specs/launcher-shell/spec.md` on macOS
- [x] 11.2 Walk every scenario in `specs/global-activation/spec.md` on macOS
- [x] 11.3 Confirm cold and warm activation meet the 80ms budget on a macOS release build
- [ ] 11.4 Walk every scenario in `specs/launcher-shell/spec.md` on Windows
- [ ] 11.5 Walk every scenario in `specs/global-activation/spec.md` on Windows
- [ ] 11.6 Confirm cold and warm activation meet the 80ms budget on a Windows release build
- [ ] 11.7 Confirm the launcher hands focus back to the previous window on Windows dismiss, including when that window belongs to another process
