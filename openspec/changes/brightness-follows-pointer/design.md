# Design

## Context

See `proposal.md` for why and `specs/display-brightness/spec.md` for behaviour. Builds on the open `display-brightness` change (see its `design.md`).

- `BrightnessMonitors::external()` enumerates every external monitor fresh on each step. The extension's `BrightnessStep::invoke` takes the bus lock, takes the pending tenths, then calls `external()` and `step_all`.
- The extension gets no config today: `[display]` is the shared `Table` (only `enabled` and `commands`).
- The launcher already finds "the display under the pointer" for its own placement (`work_area` in `platform/windows/mod.rs`, `screen_under_cursor` in `platform/macos/mod.rs`). The macOS one uses `NSScreen`, which needs the main thread; a brightness step runs on its own worker thread.

## Goals / Non-Goals

**Goals:**

- The pointer filter lives in platform code; step logic, messages, and press combining stay as they are.
- The setting is read on every step, so a config reload applies without rebuilding the extension.

**Non-Goals:**

- Sharing pointer lookup code with launcher placement (see below).
- Caching which monitor is under the pointer.

## Decisions

### Setting: `display.monitors = "all" | "pointer"`

`[display]` gets its own table type, `DisplayTable`, like `ClipboardTable`: `enabled`, `commands`, and `monitors: BrightnessMonitorsSetting` (enum `All` default, `Pointer`, lowercase in TOML). It joins the `extension_table!` list. Schema and `docs/config.md` come from the type, so the settings reference picks it up.

Name follows `launcher.display = "pointer"`: same word for the same idea. `monitors` is not a reserved key (`enabled`, `commands`).

Rejected:

- A bool `follow-pointer`: a third choice later (focused window) would need a second key.
- Separate commands ("Increase Brightness Under Pointer"): doubles the manifest and hotkeys for one preference. The user asked for a setting.

### Platform: `BrightnessMonitors::under_pointer()`

New trait method next to `external()`:

```
fn under_pointer(&self) -> Vec<Box<dyn Monitor>>;
```

Returns the external monitor(s) under the pointer, or none. A `Vec` because one Windows `HMONITOR` can hold more than one physical monitor (mirroring), and so `step_all` takes it unchanged: an empty list gives the existing "no monitor answers" failure.

- Windows: `GetCursorPos`, `MonitorFromPoint(point, MONITOR_DEFAULTTONULL)`, then `ddc_winapi::get_physical_monitors_from_hmonitor` and `unsafe ddc_winapi::Monitor::new` for each. `ddc-winapi` takes a `winapi` `HMONITOR`; the project's comes from `windows-sys`, so cast the raw pointer. A laptop panel enumerates but does not answer DDC/CI, so it fails like today.
- macOS: pointer from `CGEvent::location(CGEvent::new(None).as_deref())` and display id from the `unsafe` `CGGetDisplaysWithPoint` (out-buffer plus count). Both come from `objc2-core-graphics`; the features they need (`CGEvent`, `CGEventTypes`, `CGDirectDisplay`, `CGError`) are already on, the last two through the crate's defaults. Both are thread-safe CoreGraphics calls, so they run on the step's worker thread. Then `ddc_macos::Monitor::enumerate()` filtered to `monitor.handle().id == id`. A MacBook panel is never in `enumerate()`, so the list is empty.

The extension picks `external()` or `under_pointer()` from the setting. `FakeMonitors` in `src/extensions/display/` gets a pointer index; `NoMonitors` in `src/app/tests.rs` returns an empty list.

Rejected:

- Reuse `work_area` / `screen_under_cursor`: the macOS one needs the main thread (`NSScreen`); the Windows one returns a work area, not the `HMONITOR`. Two lines of pointer lookup per platform cost less than reshaping them.
- Enumerate all monitors and match by screen rectangle in the extension: needs monitor geometry in the trait for both crates, which neither exposes on the monitor type.
- Fall back to all monitors when the pointer's monitor does not answer: surprising, it changes monitors the user did not point at.

### Pointer read when the step runs

The pointer is read after the bus lock is taken, together with the pending tenths. Presses combined into one step go to the monitor under the pointer at that moment. Moving the pointer to another monitor while holding the hotkey moves the change with it from the next step. Reading at press time would need the pending sum per monitor, for no case the user hits.

### Config access

`DisplayExtension::new` takes the shared `Arc<RwLock<Config>>` (`FileConfig::shared()`), as `ConfigPolicy` does for clipboard, and reads `config.display.monitors` once per step, after the bus lock is taken, next to the pointer read. A press waiting behind a running step then uses the setting current when it applies. The config read lock is dropped before any DDC/CI work.

### Contracts and copy

No manifest or view protocol change. No new UI and no new strings: the pointer case reuses "Turn on DDC/CI in your monitor's menu to change brightness" and "Couldn't change monitor brightness". New text is only the setting's doc comment in the schema and reference:

- `monitors`: "Which monitors the brightness commands change. "all" changes every external monitor, "pointer" only the one under the mouse pointer. Default "all"."

## Risks / Trade-offs

- [Pointer on a laptop panel gives the DDC/CI advice, which does not help there, short of `interface-copy`'s "says what failed and what to do"] → Accepted gap to avoid new copy for a rare case on the author's desktop setup. The spec only requires a failure there, not this text, so a dedicated message later needs no spec change.
- [`Monitor::new` on Windows is `unsafe` and takes ownership of the `PHYSICAL_MONITOR`] → Its `Drop` calls `DestroyPhysicalMonitor`, same as `enumerate()`. Each handle goes into exactly one `Monitor`.
- [macOS display id from `CGGetDisplaysWithPoint` might not match `ddc_macos`'s `CGDisplay` on some setups (mirrored displays)] → Not checked; mirroring is not part of the author's setup. A mismatch gives the "no monitor answers" failure, never a wrong monitor.
