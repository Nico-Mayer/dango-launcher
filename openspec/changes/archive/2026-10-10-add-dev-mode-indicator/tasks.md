## 1. Backend

- [x] 1.1 Add an `is_development_build` invoke command in `src-tauri/src/lib.rs` returning `cfg!(debug_assertions)`, register it in `generate_handler!`, and verify `cargo check` passes
- [x] 1.2 Set the tray tooltip at the `TrayIconBuilder` site to "Dango (development build)" under `debug_assertions` and "Dango" otherwise, and verify by hovering the tray icon in `tauri dev` and in a release build

## 2. Presentation

- [x] 2.1 Add the `--dango-dev-edge` token to `src/lib/theme/tokens.css`, backed by a `--dango-palette-dev-edge` entry in both themes, and verify the token resolves to a different colour in each palette in the browser test
- [x] 2.2 Give `LauncherFrame.svelte` a `development` prop that sets `data-development` on `<main>`, with CSS that swaps the edge colour to the token and adds a one-edge-width inset shadow, and verify the frame keeps 720 by 400 pixels and a 14px radius in the fixture
- [x] 2.3 Make `LauncherFooter.svelte` render a "dev" badge after the "Dango" label, shown only inside a frame carrying `data-development`, using the amber tint tokens and keycap radius, and verify the footer stays 40px high with the badge shown
- [x] 2.4 In `App.svelte`, invoke `is_development_build` once in `onMount` next to `warmup_done`, keep the answer in state, and pass it to every `LauncherFrame`, and verify all three branches (root, pushed view, protocol error) carry the marker in the fixture

## 3. Fixtures and tests

- [x] 3.1 Mock `is_development_build` in `tests/fixtures/main.ts`, answering true when the URL has `dev=1` and false otherwise, document the switch in `tests/fixtures/README.md`, and verify existing scenes still open without the marker
- [x] 3.2 Add a browser test that opens the root and detail scenes with and without `dev=1`, asserts the `data-development` attribute and badge presence, and asserts footer height and position are unchanged, and verify `npm run test:browser` passes
- [x] 3.3 Run `npm run check` and the full `npm run test:browser` suite and verify both pass with no changed expectations in existing presentation tests

## 4. Verification on the desktop

- [x] 4.1 Run `tauri dev` on Windows, summon the launcher, and verify the amber edge and "dev" badge are visible on root search, inside a pushed view, and that the tray tooltip names the development build
- [x] 4.2 Build a release on Windows, summon it, and verify no marker, no badge, and the plain "Dango" tooltip
- [x] 4.3 Repeat 4.1 and 4.2 on macOS
- [x] 4.4 In `tauri dev`, read the printed activation latency for a few activations and verify it stays within the 80ms budget
