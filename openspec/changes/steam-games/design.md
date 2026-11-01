## Context

See proposal.md - Why. Steam game shortcuts are `.url` files with a
`steam://rungameid/<appid>` target. `applications` already keeps such targets
(`is_launchable` in `src/platform/windows/apps.rs`), so games with a shortcut
show today and games without one never do.

Steam keeps the same files on Windows and macOS:

```
<steam>/steamapps/libraryfolders.vdf       --> every library folder
<library>/steamapps/appmanifest_<id>.acf   --> appid, name, StateFlags
<steam>/appcache/librarycache/<id>/        --> icon (<sha1>.jpg),
                                               library_600x900.jpg, logo.png
```

`applications` already shows the pattern this change follows: an index
filled by a background service, a fingerprint poll for freshness, and an
icon cache (`src/extensions/applications/`).

## Goals / Non-Goals

**Goals:**

- One platform-neutral module. Steam's files and its `steam://` link are the
  same on both platforms, so no new platform trait is needed.

**Non-Goals:**

- No SQLite cache for the game list. See the decision below.
- No change to the extension manifest or the view protocol.

## Decisions

### Read the library with `steamlocate`

`steamlocate` 2.x finds Steam (registry on Windows, `~/Library/Application
Support/Steam` on macOS), parses `libraryfolders.vdf`, and yields each
library's installed apps with `appid`, `name`, and install state. It is
maintained and covers both platforms.

Rejected:

- Hand-parsing VDF with `keyvalues-parser` or `vdf-reader`: we would still
  write Steam discovery and the library walk ourselves, which is the part
  `steamlocate` already does.
- Steam Web API (`IPlayerService/GetOwnedGames`): needs an API key and
  network access, and lists owned games, not installed ones.
- Only reading Start Menu shortcuts: this is today's behaviour and the bug.

Check the 2.x API on docs.rs before coding (`steamlocate::locate`,
`SteamDir::libraries`, `Library::apps`). Pin the minor version.

### A new `games` built-in extension

`dango.games`, name "Games", with `root_items: true`, `services: true`, and
one command, "Search Games". Its module is `src/extensions/games/`, with the
Steam reading in `steam.rs` behind a small `GameSource` trait so tests use a
fake source, the same way `applications` tests use `FakeIndexer`.

Rejected: a second source inside `applications`. It would mix Steam logic
into the OS app indexer and has no home for the grid command.

### `applications` drops `steam://rungameid/` targets

`is_launchable` returns false for a target starting with
`steam://rungameid/`, so each game appears once, from `games`. Steam itself
(`steam.exe`) and other `steam://` links stay. Windows only for now. Whether Steam on macOS puts
per-game bundles in the folders `applications` scans is unverified, so a
macOS hand check looks for duplicates.

Rejected: deduplicating in root search by matching app ids across
extensions. It couples two providers for a case one filter removes.

### Non-games come from a short deny list

A const list of app ids, starting with `228980` (Steamworks Common
Redistributables). The real app type lives only in the binary
`appcache/appinfo.vdf`, whose format changes with Steam updates.

Rejected: parsing `appinfo.vdf`. More code and a format we do not control,
for a list that holds one entry on the author's machine.

### No persisted cache

The list is read in the background service on start. Reading a dozen small
`.acf` files is cheap, so Dango does not store the games in SQLite. Root
search shows games once the first read finishes, shortly after startup.

Rejected: a `games` table like the apps index. Extra schema and code to
cover a few milliseconds after start.

### Freshness by fingerprint poll

The service reuses the `applications` loop shape: read, then wait until a
fingerprint changes or the sweep interval passes. The fingerprint hashes the
modification times of `libraryfolders.vdf` and each library's `steamapps`
folder. Steam adds or removes an `appmanifest_*.acf` file there on install
and uninstall, which changes the folder's mtime. A launch of an id that is
no longer listed re-reads once before failing, as `applications` does.

### Launch through `steam://rungameid/<appid>`

Opened with the platform's URL opener (`open` on macOS, `ShellExecuteW` on
Windows), behind the existing open helper. Steam handles a closed client,
updates, and an already running game.

Rejected: running the game's executable directly. It skips Steam's launch
options, DRM, and overlay, and the executable is not in the manifest.

### Icons and cover art come from the library cache, read lazily

- Root result icon: the `<sha1>.jpg` in `librarycache/<appid>/` that is not
  one of the named files. Steam names the icon by hash, so pick the single
  remaining `.jpg`. Missing: `icon:gamepad-2`.
- Grid tile: `library_600x900.jpg`, as an image path. Missing:
  `icon:gamepad-2`.

Both are image paths given straight to the view, so no extraction or icon
cache is needed. Slint builds with `image-default-formats`, which loads JPEG.

### UI

- Root search: list rows (`ui/views/` list) like any application result.
- "Search Games": the existing grid view (`ui/views/` grid) with 5 columns
  and `aspect` 2/3, which fits Steam's 600x900 cover. Its filter is the
  standard SearchBar filter. No new component.
- Add `gamepad-2` (`LuGamepad2`, present in `icondata_lu` 0.1) to the
  registry in `src/slint_shell/icons.rs`, and use it as the extension icon.

### Interface copy

Written with the ux-writing skill, checked against
`openspec/specs/interface-copy/spec.md`:

| Where | Text |
|---|---|
| Extension name | "Games" |
| Command title | "Search Games" |
| Root result subtitle | "Steam" |
| Primary action (root and grid) | "Open" |
| Grid empty state title | "No Steam games installed" |
| Grid empty state description | "Games you install with Steam will show up here" |
| Toast, game no longer listed | "That game is no longer installed" |
| Toast, Steam link failed to open | "Couldn't start the game through Steam" |

"Open" follows the rule that launching uses "Open". The empty state matches
clipboard history's "will show up here".

## Risks / Trade-offs

- [The `steamlocate` 2.x API differs from the names above] -> Check docs.rs
  before the first task. The design depends on what it reads, not on its
  function names.
- [Steam's cache file names change] -> A missing file falls back to the
  gamepad icon, so a change costs looks, not launches.
- [Deny list misses a tool such as a dedicated server or SDK] -> Add the id
  when it shows up. Cheap, and specific to the author's library.
- [Folder mtime does not change on some file systems] -> The sweep interval
  still catches the change, only later.
