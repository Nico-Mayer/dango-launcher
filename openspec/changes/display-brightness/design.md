# Design

## Context

See `proposal.md` for why and `specs/display-brightness/spec.md` for behaviour.

- Commands run on their own thread (`Invoker::invoke` in `src/invocation.rs` spawns one per run). A newer run supersedes an older one: the older run's output is dropped, but its thread keeps going unless it checks `is_cancelled`.
- A no-view command finishes with `Outcome::Success` or `Outcome::Failure(String)`. `src/app/mod.rs` turns success into `hide_after_launch` and failure into `ui::Event::Failed`, which shows a ticker or, while hidden, the notice. There is no way to report a success message today; only actions can (`ActionOutcome::Ticker`, the copy notice in `src/ui/state.rs`).
- DDC/CI is slow. Each read or write talks to the monitor over I2C, and monitors need a pause between messages. A step is a read followed by a write, per monitor.

## Goals / Non-Goals

**Goals:**

- Step logic (range, rounding, clamping, message text, partial failure) is platform-neutral and covered by `cargo test` with fake monitors.
- Platform code only finds monitors and hands back something that can read and write VCP `0x10`.

**Non-Goals:**

- Caching monitor handles or levels between runs.
- Capability-string parsing (`mccs-caps`) to check whether a monitor supports brightness before trying.

## Decisions

### Crates: `ddc-winapi` and `ddc-macos` over the shared `ddc` 0.2 traits

Both implement `ddc::Ddc` (`get_vcp_feature`, `set_vcp_feature`) for a `Monitor` type with `Monitor::enumerate()`. The returned value carries the current level and the monitor's maximum, so the step uses each monitor's real range.

- `ddc-winapi` 0.2.2 (2024-07): wraps the Dxva2 monitor configuration API (`GetPhysicalMonitorsFromHMONITOR`, `GetVCPFeatureAndVCPFeatureReply`, `SetVCPFeature`). Built on `winapi` 0.3, which is already in `Cargo.lock` (0.3.9), so it adds no new crate besides itself.
- `ddc-macos` 0.2.2 (2024-11): Intel path through IOKit I2C, Apple Silicon path through the private `IOAVService` read/write calls (`src/arm.rs`).
- Both depend on `ddc` 0.2, so Dango pins `ddc` 0.2 (not the newer 0.3) to share one trait.

Rejected:

- `ddc-hi` 0.4.1: last release 2021-11. Pulls `nvapi`, `ddc-i2c`, `edid`, `mccs-caps`, `mccs-db` for capability parsing and EDID names Dango does not need. Its only gain is one enumerate call over both backends, which a ten-line `cfg` pair in `platform/` gives too.
- Own Dxva2 code on the `windows` crate (what rusty-twinkle-tray does): `ddc-winapi` does the job, so the project rule says use it.
- `ddc-winapi` 0.3.0-alpha.2: pre-release, and moves to `ddc` 0.3, which `ddc-macos` does not use.

### Platform trait returns monitors; the extension does the stepping

New trait in `src/platform/mod.rs`, next to `SystemControl`:

```
trait BrightnessMonitors: Send + Sync {
    fn external(&self) -> Vec<Box<dyn Monitor>>;
}
trait Monitor {
    fn name(&self) -> &str;
    fn brightness(&mut self) -> Result<Level, MonitorError>;   // value + maximum
    fn set_brightness(&mut self, value: u16) -> Result<(), MonitorError>;
}
```

One adapter generic over `ddc::Ddc` implements `Monitor` for both crates, so the per-platform code is only the `external()` constructor: `ddc_winapi::Monitor::enumerate()` on Windows, `ddc_macos::Monitor::enumerate()` on macOS. `description()` is an inherent method on each crate's `Monitor`, not part of `ddc::Ddc`, so `external()` reads it and passes the name into the adapter.

`Monitor` has no `Send` bound: both crates' monitor types wrap raw OS handles (`PHYSICAL_MONITOR` on Windows, `IOAVService` as `CFTypeRef` on macOS) and are not `Send`. Monitors are enumerated, used, and dropped on the thread that runs one step.

Built-in panels: `ddc-macos` already skips displays where `is_builtin()` is true (`arm.rs`, `intel.rs`). On Windows, a laptop panel enumerates too but does not answer DDC/CI, so it falls out as a monitor that "does not answer".

`src/extensions/display/` holds the manifest, the two commands, and a pure `step(levels, direction) -> plan` function plus the message builder, tested without hardware.

Rejected: a trait method `step(direction)` per platform. It would duplicate clamping, rounding, and message rules on both sides and leave them untestable.

### Step size and percentages

Step = one tenth of the monitor's maximum, rounded, at least 1. New value = current plus or minus step, clamped to `0..=maximum`. Shown percentage = new value times 100 divided by maximum, rounded. A monitor with maximum 100 (the common case) moves exactly 10%. A monitor that reports a maximum of 0 counts as one that does not answer, so the percentage never divides by zero.

### Repeated presses: one lock per step, ignore superseding

The spec requires every press to count and no two steps to overlap. Two facts decide this:

- A second press supersedes the first run, so the first run's result is dropped. If the first run gave up on `is_cancelled`, quick presses would be lost.
- Two runs talking to the same monitor at once interleave I2C messages, which many monitors answer with garbage or not at all.

So each command run takes one `static Mutex<()>` in the extension and holds it for the whole step: enumerate, read, write. The command does not check `is_cancelled`. Each step reads fresh levels, so same-direction presses give the same result in any order. A superseded run's notice is dropped by the existing sink, so after quick presses only the last run's message shows.

Trade-off: `std::sync::Mutex` is not FIFO. An up press and a down press within one DDC round trip may apply in swapped order. The end level is the same unless a clamp sits between them, which is not worth more machinery.

Rejected:

- Drop presses while one is running: loses presses, breaks "each counts".
- A worker thread fed by a channel: strict order, but a thread, a channel, and a reply path for an ordering difference no one will notice.
- Coalescing queued presses into one step of `n` tenths: fewer bus round trips, but more state for a gain only noticeable with many presses. Revisit if holding a hotkey feels slow.

### Success with a message

`Outcome::Success` becomes `Outcome::Success(Option<String>)`, with `InvocationContext::succeed_with(message)` next to `succeed()`. In `src/app/mod.rs`, success with a message hides the launcher as today, then applies a new `ui::Event` that calls the existing `show_notice(TickerStyle::Success, ...)`, as the copy path does. This works the same from root search and from a hotkey: either way the launcher is hidden when the notice shows.

Rejected: an `ActionOutcome`-style ticker that keeps the launcher open. No-view success closes the launcher by spec, and from a hotkey there is no launcher to keep open.

Contract note: this widens the no-view outcome in `view-protocol`. The change is additive, and the project is in alpha, so the protocol version does not change. Third-party extensions do not exist yet.

### Extension placement: new `display` extension

A separate `display` extension, like `window-management`, rather than two more commands in `system`. It keeps `system-commands` unchanged, gives later display features (a monitor list view, built-in panels) a home, and lets the user disable it on a desktop without DDC/CI monitors. Hotkeys live at `display.commands.increase-brightness.hotkey` and `display.commands.decrease-brightness.hotkey`.

### Surfaces and copy

No new UI component. Output goes through the existing notice pill (`notice` spec) and failure ticker. Command icons come from the `icondata_lu` registry: `sun` for increase and `sun-dim` for decrease, added to `src/slint_shell/icons.rs` if missing.

Strings, written with the ux-writing skill and checked against `interface-copy`:

- Command titles: "Increase Brightness", "Decrease Brightness" (Title Case like other command titles; verb and object).
- Keywords: "brightness", "brighter", "dim", "dimmer", "monitor", "display", "screen".
- Success, one level: "Brightness set to 60%".
- Success, different levels: "Brightness set to 40% and 70%"; three or more: "Brightness set to 30%, 50%, and 70%".
- No monitor answers: "Turn on DDC/CI in your monitor's menu to change brightness".
- Every monitor refuses: "Couldn't change monitor brightness".

Library errors go to `dango.log` with the monitor's name, never into the ticker.

## Risks / Trade-offs

- [A monitor's DDC/CI is off by default, or its connection does not carry DDC/CI (some docks, DisplayLink, the HDMI port on some Apple Silicon Macs)] → The "no monitor answers" failure names the most common fix. Ports and docks are a hardware limit Dango accepts.
- [A run takes a noticeable moment per monitor] → It runs off the main thread and never blocks the launcher; only the notice arrives late.
- [`ddc-macos` uses private `IOAVService` calls] → No public API exists for this on Apple Silicon. Every DDC tool on macOS does the same. If a macOS update breaks it, the command fails with the message above and nothing else is affected.
- [`ddc-winapi` is passively maintained on `winapi` 0.3] → The API it wraps has been stable since Windows Vista. If it falls behind, swap it for own code on the `windows` crate behind the same trait.
- [Enumerating monitors on every run costs time] → Keeps hot-plugging correct without any watcher. Revisit only if a step feels slow.
- [The macOS hardened runtime or a permission might block `IOAVService` access] → Unknown until tried on a signed build; covered by a manual task.
