## Why

Root search finds a Steam game only when Steam created a Start Menu shortcut
for it. On the author's machine that is 3 of 13 installed games, so most games
cannot be launched from Dango.

## What Changes

- New built-in `games` extension that reads the installed Steam library
  directly and contributes each game to root search.
- Confirming a game launches it through Steam.
- Each game shows its own icon from Steam's local library cache, with a
  gamepad icon when none is cached.
- New "Search Games" command: a grid of installed games with their cover art.
- Steam tools that are not games, such as the Steamworks redistributables, are
  left out.
- The `applications` extension stops listing Steam game shortcuts, so a game
  with a shortcut appears once, from `games`.

## Capabilities

### New Capabilities

- `games`: discovering installed Steam games, showing them in root search and
  in a cover grid, and launching them through Steam.

### Modified Capabilities

- `applications`: Steam game shortcuts are excluded from discovery on Windows.

## Platforms

Windows and macOS. Steam keeps the same library files on both, and both
launch a game through the same Steam link.

## Non-goals

- Other stores: Epic, GOG, Battle.net, EA, Ubisoft. Xbox games stay in
  `applications`, which already lists them.
- Games that are owned but not installed.
- Non-Steam games the user added to Steam by hand.
- Steam account data: playtime, achievements, friends, store prices.
- Installing, updating, or uninstalling games.
- Linux.

## Impact

- New module `src/extensions/games/` and its registration.
- `src/platform/windows/apps.rs`: `is_launchable` drops Steam game targets.
- New dependency: `steamlocate`.
- One new named icon in `src/slint_shell/icons.rs` if the registry has no
  gamepad icon yet.
