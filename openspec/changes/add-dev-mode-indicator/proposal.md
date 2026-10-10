## Why

A `tauri dev` run and the installed release look identical: same window, same
footer, same tray icon. When both exist on the machine the author cannot tell
from the launcher itself which one just answered the hotkey, so a change under
test gets judged against the wrong build, or a stale dev process is left
running by mistake. A development build already behaves differently in two
places (autostart is ignored, activation latency is printed), but nothing on
screen says so.

## What Changes

- The launcher window shows a visible development marker in a development
  build: the frame edge takes a distinct development colour, and the footer's
  "Dango" label gains a small "dev" badge. A release build shows neither.
- The tray icon's tooltip names the development build, so two resident Dango
  processes can be told apart without summoning either.
- "Development build" keeps the meaning the project already uses for autostart
  and the latency probe: a debug build. The backend is the single source of
  that answer; the renderer only asks once at mount and paints accordingly.
- The browser fixtures can mount either state, so both appearances are
  covered by the Playwright suite without a Tauri process.

Milestone: M7 polish. Platforms: Windows and macOS, no platform-specific code.

## Non-goals

- Separating the development build's config directory, SQLite store, or
  application identifier from the release build. The two still share them.
- Letting a development build and a release build run at the same time. The
  single-instance guard is unchanged.
- A different tray icon image for the development build, or any tray menu
  change beyond the tooltip.
- A user-facing setting to turn the marker on or off, or a theme file.
- Showing build version or commit information anywhere in the interface.

## Capabilities

### New Capabilities

- `development-indicator`: how a development build of Dango announces itself
  in the launcher window and the tray, and how a release build does not.

### Modified Capabilities

None. The existing launcher-shell requirement that the window has no system
chrome is untouched: the marker is drawn inside the webview, on the frame
edge that already exists.

## Impact

- `src-tauri/src/lib.rs`: one new invoke command answering whether this is a
  development build, and a tray tooltip set from the same answer.
- `src/App.svelte`, `src/lib/launcher/LauncherFrame.svelte`,
  `src/lib/launcher/LauncherFooter.svelte`: ask once at mount, pass the
  answer down, render the edge colour and the badge from it.
- `src/lib/theme/tokens.css`: one new token for the development edge colour,
  so the marker follows the shared presentation like every other surface.
- `tests/fixtures/main.ts` and `tests/browser/`: mock the new command, add a
  scene switch for the development state, and a test for both states.
- No new dependencies. No config or store changes.
