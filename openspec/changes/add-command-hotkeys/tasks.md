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

- [ ] 4.1 Confirm on both platforms that a command bound in the config runs from its hotkey while another application is focused, and that a no-view command does so without the launcher appearing
  - Windows: with a plain target window focused, ctrl+alt+left ran
    window-management left-half on it (its frame became the exact left half of
    the work area) while the launcher window stayed hidden. macOS pending.
- [ ] 4.2 Confirm on both platforms that a view command's hotkey shows the launcher with its view
  - Windows: ctrl+alt+v, bound to clipboard history (a view command), showed the
    launcher window. macOS pending.
- [ ] 4.3 Confirm on both platforms that a snippet or window-management hotkey acts on the window that was focused when the chord was pressed
  - Windows: the window-management hotkey moved the focused target window, not
    the launcher, so it acted on the window focused at the press. macOS pending.
- [ ] 4.4 Confirm on both platforms that adding, changing, and removing a binding in the file applies live
  - Windows: rewriting config.json while running rebound left-half to ctrl+alt+up
    (which then moved the target) and dropped right-half's ctrl+alt+right (which
    then did nothing), no restart. macOS pending.
- [ ] 4.5 Confirm on both platforms that two commands on one chord, a command on the launcher chord, and an OS-refused chord are each surfaced, with the first binding winning and the others reported
  - Windows: dango.log carried all three - top-half and left-half on one chord
    with left-half keeping it, maximize on the launcher chord, and center refused
    by the OS (the chord was pre-claimed by the harness). macOS pending.
