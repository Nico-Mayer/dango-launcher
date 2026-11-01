## 1. Steam library source

- [x] 1.1 Check the `steamlocate` 2.x API on docs.rs against design.md and add it to `Cargo.toml` with the minor version pinned; if it cannot list installed apps per library with name and install state, stop and update design.md; verify `cargo build` passes
- [x] 1.2 Add `src/extensions/games/steam.rs` with a `GameSource` trait (list games, fingerprint) and a `SteamSource` that walks every library, keeps fully installed apps, drops the deny list (`228980`), and finds each game's icon (the hash-named `.jpg`) and cover (`library_600x900.jpg` or `library_capsule.jpg`, top level or in a hash subfolder) in `appcache/librarycache/<appid>/`; verify with tests in `steam.rs` over a temp folder that mimics Steam's layout: two libraries, a deny-listed app, a game with no cached art, a cover in a subfolder, and a fingerprint that changes when a manifest is added (`cargo test games`)

## 2. Games extension in root search

- [x] 2.1 Add `gamepad-2` (`LuGamepad2`) to the registry in `src/slint_shell/icons.rs`; verify the registry's own tests pass (`cargo test icons`)
- [x] 2.2 Add `src/extensions/games/mod.rs`: `dango.games` manifest named "Games" with `root_items` and `services`, an in-memory game list, a root provider that yields each game with subtitle "Steam", its icon or `icon:gamepad-2`, and primary action "Open"; register it in `src/extensions/mod.rs` and in startup; verify with tests using a fake `GameSource` for the results, the fallback icon, and `every_built_in_manifest_validates` (`cargo test games`)
- [x] 2.3 Add the background service: read on start, then wait for a fingerprint change or the sweep interval, reusing the `applications` loop shape; verify with a test that a changed fake fingerprint triggers a re-read (`cargo test games`)
- [x] 2.4 Launch reads the list, then opens `steam://rungameid/<appid>` through the existing URL opener and hides the launcher; an id no longer listed fails with "That game is no longer installed"; an opener error fails with "Couldn't start the game through Steam"; verify with tests on the fake source for both failures and the opened URL, and add both messages to the fixed-message sentence test (`cargo test games`)

## 3. Search Games grid

- [x] 3.1 Add the "Search Games" command: a grid with 5 columns, `aspect` 2/3, each tile's content the cover path or `icon:gamepad-2`, primary action "Open", empty state "No Steam games installed" / "Games you install with Steam will show up here"; verify with extension tests for the tiles and the empty state (`cargo test games`)
- [x] 3.2 UI test in `src/slint_shell/tests.rs`: run "Search Games" with a fake source, type part of a name, confirm only matching tiles remain, press Enter, and confirm the launch request reaches `dango.games` and opens the game; the hidden launcher is a `Core` test in `src/app/tests.rs`, since hiding after a launch is `Core`'s (`cargo test slint_shell`, `cargo test app`)

## 4. No duplicates from applications on Windows

- [x] 4.1 In `is_launchable` in `src/platform/windows/apps.rs`, return false for targets starting with `steam://rungameid/`; replace `custom_url_schemes_are_launchable` with a test that keeps another `steam://` link and drops a game link, and keep a test that `steam.exe` stays (`cargo test apps`)

## 5. Hand checks on Windows

- [ ] 5.1 Start a debug build with the Slint MCP server; with `scripts/send-keys.ps1` open the launcher and type "cyber", and confirm with `scripts/screenshot.ps1` that Cyberpunk 2077 (no shortcut) shows once with its icon; type "brawl" and confirm Brawlhalla (has a shortcut) shows once; confirm "Steamworks" shows nothing
- [ ] 5.2 Close Steam, launch a game from root search, and confirm Steam starts and starts the game
- [ ] 5.3 Run "Search Games" and confirm with a screenshot that the covers fill portrait tiles without stretching

## 6. Hand checks on macOS

- [ ] 6.1 Confirm `cargo test` passes on a Mac
- [ ] 6.2 With Steam installed on the Mac, open the launcher, search for an installed game, and confirm it shows once (no duplicate from `applications`) with its icon, and launches through Steam
- [ ] 6.3 Run "Search Games" and confirm the covers show

## 7. Wrap up

- [x] 7.1 Run `hk check` and `cargo test`; verify both pass
- [ ] 7.2 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context; it reports findings only and the author decides what to fix
