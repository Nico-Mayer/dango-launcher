## 1. The binding table and conflict detection

- [x] 1.1 Build the command-binding table from a config: parse each `extensions.<id>.commands.<id>.hotkey`, resolve conflicts first-wins against other commands and against the launcher chord, and return both the survivors and the conflicts, with tests over a clean set, two commands on one chord, and a command on the launcher chord
- [x] 1.2 Factor the command-run streaming out of `invoke_command` into a shared `run_command(app, qualified_id)` used by both the Tauri command and the hotkey dispatch, and verify the existing invocation still works

## 2. Registering and dispatching

- [x] 2.1 Register the survivors' hotkeys at startup alongside the launcher hotkey, holding the shared table the handler reads
- [x] 2.2 Dispatch in the global-shortcut handler: the launcher chord toggles, a bound chord runs its command, an unknown chord is ignored, with the launcher chord checked first
- [x] 2.3 Route by mode: a no-view command runs without showing the launcher, a view command shows the launcher then invokes, read from the command's `InvocationMode`
- [x] 2.4 Before a no-view command, record the current foreground as the previous window on Windows so the command acts on the window that was focused at the press; macOS needs nothing, since the frontmost application is already the target
- [x] 2.5 Surface the conflicts from 1.1 and any OS registration refusal through the tray status line and the log, naming the commands involved

## 3. Live reload

- [x] 3.1 Extend the config reload to unregister the current command hotkeys, rebuild the table, and register the survivors, leaving the launcher hotkey untouched unless it changed, with the conflicts re-surfaced
- [x] 3.2 Verify the existing test suite passes and clippy and fmt are clean

## 4. Verification

- [x] 4.1 Confirm on both platforms that a command bound in the config runs from its hotkey while another application is focused, and that a no-view command does so without the launcher appearing
  - Windows: with a plain target window focused, ctrl+alt+left ran
    window-management left-half on it (its frame became the exact left half of
    the work area) while the launcher window stayed hidden.
  - macOS: with the harness's own window focused, ctrl+alt+i ran right-half and
    ctrl+alt+u ran left-half on it, each landing on the exact half of the work
    area, while the launcher window count stayed 0 throughout - a no-view
    command runs without the launcher appearing.
- [x] 4.2 Confirm on both platforms that a view command's hotkey shows the launcher with its view
  - Windows: ctrl+alt+v, bound to clipboard history (a view command), showed the
    launcher window.
  - macOS: ctrl+alt+v, bound to clipboard history (a view command), brought the
    launcher up (window count 1) and left the focused window where it was.
- [x] 4.3 Confirm on both platforms that a snippet or window-management hotkey acts on the window that was focused when the chord was pressed
  - Windows: the window-management hotkey moved the focused target window, not
    the launcher, so it acted on the window focused at the press.
  - macOS: both window-management hotkeys moved the window that was focused when
    the chord was pressed, not the launcher. On macOS the headless foreground
    capture is a no-op, since the frontmost application is the target, and that
    path is what these runs exercised.
- [x] 4.4 Confirm on both platforms that adding, changing, and removing a binding in the file applies live
  - Windows: rewriting config.json while running rebound left-half to ctrl+alt+up
    (which then moved the target) and dropped right-half's ctrl+alt+right (which
    then did nothing), no restart.
  - macOS: all three applied live with no restart. Removing left-half's hotkey
    left ctrl+alt+u doing nothing while ctrl+alt+i kept working; rebinding
    left-half to ctrl+alt+o made the new chord move the window.
  - macOS note: ctrl+alt+<arrow> is claimed by the system for Spaces and never
    reaches Dango, which is why the letter chords are used here. Registration
    still succeeds, so this is the OS winning at dispatch, not a refusal.
- [x] 4.5 Confirm on both platforms that two commands on one chord and a command on the launcher chord are each surfaced, with the first binding winning and the others reported, and that a chord the OS refuses is surfaced on the platforms where the OS refuses one
  - Windows: dango.log carried all three - top-half and left-half on one chord
    with left-half keeping it, maximize on the launcher chord, and center refused
    by the OS (the chord was pre-claimed by the harness).
  - macOS: two of the three were surfaced in `dango.log` - top-half and
    left-half on one chord with left-half keeping it, and maximize on the
    launcher chord with the launcher keeping it.
  - macOS: the OS-refusal case does not exist on this platform. Carbon's
    `RegisterEventHotKey` accepted every chord tried that the system already owns
    (cmd+tab, ctrl+f2, cmd+shift+3, cmd+alt+esc, cmd+space) and reported no
    error; the system wins at dispatch instead, where Windows' `RegisterHotKey`
    fails outright. A chord another owner holds is therefore not refused on
    macOS and there is nothing to surface, which the spec now says with a
    scenario per platform rather than one flat requirement.