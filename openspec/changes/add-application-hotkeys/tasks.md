## 1. Config model

- [x] 1.1 Add `AppSettings { name: Option<AppName>, hotkey: Option<Hotkey>,
      extra }` and `apps: BTreeMap<String, AppSettings>` to `config::Extension`
      in `src-tauri/src/config/mod.rs`, where `AppName` is a string or a
      `{ macos, windows }` object resolved for the running platform; verify with
      tests that both shapes parse, that an entry with no name for this platform
      resolves to `None`, and that a file with `apps` round-trips unchanged.
- [x] 1.2 Document the branch in `docs/config.schema.json` (an `apps` map on
      the extension definition, with `name` and `hotkey`) and in
      `docs/config.example.jsonc` with one plain entry and one per-platform
      entry; verify the example still parses as JSONC and `Config::parse`
      accepts the equivalent JSON.

## 2. One binding table for commands and applications

- [x] 2.1 In `src-tauri/src/hotkeys.rs`, introduce `Target::{Command,
      Application}` and rename `command_bindings` to `bindings`, iterating each
      extension's `commands` then `apps`; verify the existing tests pass with
      the new shape and new tests cover an application binding, an application
      and a command on one chord reported by name, and an entry skipped because
      its name is for the other platform.
- [x] 2.2 Update `apply_command_hotkeys` in `src-tauri/src/lib.rs` to register
      and hold `(Shortcut, Target)`; verify `cargo clippy -- -D warnings` is
      clean and the app starts with existing command hotkeys still working.

## 3. Launching from a press

- [x] 3.1 Add `find_by_name(&self, name: &str) -> Vec<IndexedApp>` to
      `ApplicationsExtension` (or `AppIndex`), case-insensitive, sorted by
      name; verify with tests using `FakeIndexer` for a single match, no match,
      and two matches.
- [x] 3.2 In `src-tauri/src/lib.rs`, dispatch `Target::Application` by
      resolving the name, performing `ACTION_LAUNCH` through the host,
      crediting frecency on `Done`, hiding a visible launcher, and sending a
      missing-application, ambiguity, or launch failure through `log_config`
      and the tray status; verify with a unit test around the resolution and
      message building where possible, and by reading `dango.log` after a press
      on a name that is not installed.
  - Resolution and message building are unit tested. On macOS, pressing the
    temporary `Missing app check` binding launched nothing, and `dango.log`
    recorded `application "Missing app check" is bound to a hotkey but is not
    installed`.

## 4. Verify on macOS

- [x] 4.1 Bind `Safari` to a hyper chord, press it with another app focused,
      and confirm Safari opens without the launcher appearing; press it again
      with Safari running and confirm its window comes forward with no second
      instance.
  - Confirmed by the author on macOS.
- [x] 4.2 Add a binding while Dango runs and confirm it works without a
      restart; remove it and confirm the chord is released.
  - Confirmed by the author on macOS: both live registration and release worked.
- [x] 4.3 Bind a name that is not installed, press it, and confirm nothing
      launches and the tray status and `dango.log` name the entry.
  - Confirmed with `Missing app check` on `hyper+u`; nothing launched and the
    log named the entry. The author confirmed the tray status changed.
- [x] 4.4 Bind an application and a command to one chord and confirm the first
      in the configuration's deterministic key order wins and the tray reports
      both.
  - Confirmed with Safari and `dango.window-management.left-half` on `hyper+h`.
    The tray reported both and Safari won, matching extension-key order.
- [x] 4.5 Use a `name: { macos, windows }` entry and confirm the macOS name is
      the one matched.
  - Confirmed by the author with Safari as the macOS name.
- [x] 4.6 Confirm a hotkey launch raises the application's frecency in root
      search.
  - Confirmed directly in `dango.sqlite`: Safari's launch count increased from
    1 to 2 after one `hyper+h` launch.

## 5. Verify on Windows

- [x] 5.1 Repeat 4.1 through 4.4 with a Store application and a desktop
      application, confirming the focused window is not disturbed until the
      launched app takes the foreground.
  - Confirmed on a debug build with the launcher running. `Rechner` (Store) on
    `hyper+b`: two presses left exactly one `CalculatorApp` process and the
    launcher never appeared. `Zed` (desktop) on `hyper+z`: two presses brought
    it forward with its process count unchanged. The bindings were added to
    `config.json` while Dango ran and worked without a restart; restoring the
    file released the chords, so `hyper+b` did nothing and `hyper+left` tiled
    again. `Missing app check` on `hyper+u` launched nothing and `dango.log`
    recorded `application "Missing app check" is bound to a hotkey but is not
    installed`. `Brave` and `dango.window-management.left-half` on `hyper+left`:
    Brave won, the log said `application "Brave" keeps it`, and the tray showed
    "Config error" and "Hotkey conflict".
- [x] 5.2 Use the same `name: { macos, windows }` entry as 4.5 and confirm the
      Windows name is the one matched.
  - Confirmed with `{ "macos": "TextEdit", "windows": "Editor" }` on `hyper+e`:
    Notepad opened (shown as `Editor` on a German Windows), one process, so the
    Windows name was the one matched. Note the shown name of Windows Terminal is
    `Terminal` in `shell:AppsFolder`, not `Windows Terminal`.
- [x] 5.3 Confirm CI is green on `windows-latest` and `macos-latest`, including
      clippy, before the change is considered done.
  - Confirmed by the author on the run for main at `2702ada`: both jobs green,
    clippy included. The run before it needed a rustfmt fix on two clipboard
    files that had landed unformatted from macOS.
