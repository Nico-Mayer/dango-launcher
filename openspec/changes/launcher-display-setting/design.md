# Design

## Context

See proposal.md - Why. Today both platforms pick the display in one helper
each: `cursor_work_area` in `src/platform/windows/mod.rs` and
`screen_under_cursor` in `src/platform/macos/mod.rs`. The launcher
(`LauncherWindow::position_on_active_display`) and the notice
(`NoticeWindow::place`) both call it, from `SlintShell::show` and from the
notice's show in `publish`.

The shell is built before the config is loaded (`app/startup.rs`), and Core
owns the live config (`FileConfig`, replaced in `apply_config_reload`).

## Goals / Non-Goals

**Goals:**

- One value decides the display for the launcher and the notice, so they never
  disagree.
- The platform code only picks a display; Core decides which one is wanted.

**Non-Goals:**

- No change to how the launcher is centred or sized on a display
  (`launcher_origin`, `notice_origin` stay as they are).

## Decisions

### A `LauncherDisplay` enum in config

`LauncherDisplay { Pointer, Primary }` in `src/config/mod.rs`, serde
`rename_all = "lowercase"`, with `JsonSchema`, like `ProviderKind`. Field
`display: Option<LauncherDisplay>` on `Launcher`, included in `is_empty`. The
same type travels to the shell and platform code, so there is no second enum
to map.

Named `LauncherDisplay`, not `Display`, to avoid clashing with
`std::fmt::Display`.

Doc comment, which becomes the schema and `docs/config.md` description:

> The display the launcher and its notice open on. "pointer" is the display
> under the mouse pointer, "primary" is your main display. Default "pointer".

Rejected: a bool such as `on-primary = true`. Adding an active-window option
later would then need a rename. An enum takes one more value.

### Core pushes the value to the shell

New `Shell::set_display(LauncherDisplay)`. Core calls it once at start,
before the hotkey handler is registered (`set_event_handler` in
`app/startup.rs`), and in `apply_config_reload` after the config is
replaced. `SlintShell` stores it (atomic or `Mutex`, called from any thread),
and passes it to `position_on_active_display(handle, display)` and
`NoticeWindow::place(handle, display)`. The recording fake in tests records
the value.

Rejected alternatives:

- `Shell::show(display)`: covers the launcher only. The notice is placed from
  `publish`, which would need the value by a second path.
- Give `SlintShell` the config handle: the shell exists before the config is
  loaded, and the shell would then depend on the config file shape. Today Core
  is the only reader.
- Put the display on the `Snapshot`: the snapshot is the state machine's UI
  state, and the display is not part of it.

### Picking the primary display per platform

- Windows: `MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY)`.
  The primary monitor always has its top-left at (0, 0) in virtual-screen
  coordinates. Same DPI and work-area code as today.
- macOS: `NSScreen::screens()` index 0, the main display set in System
  Settings > Displays, whose frame origin is (0, 0). One `primary_screen()`
  helper returns it.

Rejected on macOS: `NSScreen::mainScreen`. It is the screen holding the key
window, not the main display, so it can move with focus.

With no display under the pointer, the pointer path falls back to the primary
display. On Windows `MONITOR_DEFAULTTOPRIMARY` already does this. On macOS
`screen_under_cursor` falls back to `mainScreen` today; it changes to
`primary_screen()`, so both platforms fall back the same way.

No crate needed: the choice is one system call per platform, beside code that
already calls these APIs.

## Risks / Trade-offs

- [The notice shows before Core has called `set_display`] -> `SlintShell`
  starts with `Pointer`, today's behaviour, and `set_display` runs before the
  hotkey handler is registered, so no hotkey can show the launcher first.
- [Primary display changes while running] -> nothing cached: the display is
  looked up on every show.

## Migration Plan

None. A missing key means `"pointer"`, today's behaviour.
