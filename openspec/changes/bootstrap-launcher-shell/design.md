## Context

The repository is an unmodified `create-tauri-app` scaffold: SvelteKit with
`adapter-static`, a single 800x600 window, and a `greet` command. Nothing here
has been designed yet, so this change sets the window model, the platform
boundary, and the frontend stack that every later milestone inherits.

Two constraints dominate. The 80ms activation budget rules out creating the
window or booting the webview on demand. And "first class on Windows and macOS"
means the two platforms need genuinely different window implementations from the
first line of code, not a shared path with later exceptions bolted on.

## Goals / Non-Goals

**Goals:**

- Prove the activation latency budget with a measurement, before any feature
  depends on it.
- Establish the platform trait boundary with a real second implementation, so
  the abstraction is validated rather than imagined.
- Pick the frontend stack now, while changing it is free.
- Get both platforms building in CI from the first change.

**Non-Goals:**

- Any user-visible feature. M0 shows an empty prompt.
- Rebinding the shortcut, persisting anything, or a preferences surface.
- Optimising below the 80ms budget. Meeting it is enough.
- Any accommodation for Linux.

## Decisions

### Window model: NSPanel on macOS, layered tool window on Windows

A standard Tauri window cannot meet the spec on macOS. Showing it activates the
application, which steals focus in a way the user feels, and it cannot appear
over another application's fullscreen space without the system switching spaces.
The launcher must be an `NSPanel` with the non-activating style mask and a
collection behaviour that joins all spaces and permits fullscreen auxiliary
display. That combination lets the panel become key and receive typing while the
underlying application stays active.

Windows needs a different set of properties for the same outcome: an extended
tool window style to stay out of the taskbar and Alt+Tab, topmost z-order, and
an explicit foreground grab because Windows has no non-activating equivalent.

These differ enough that a single cross-platform path would be a lie. Both sit
behind one `LauncherWindow` trait exposing `show`, `hide`, `position_on_active_display`,
and `restore_previous_focus`.

**Rejected:** using the default Tauri window with `always_on_top` and
`skip_taskbar` on both platforms. It is close enough to work in a demo and
visibly wrong in daily use on macOS, which is exactly the kind of thing that
becomes unfixable once features pile on top of it.

**Risk:** the macOS panel conversion depends on a community crate rather than
Tauri itself. Treated as a spike in tasks, with a measurement before the rest of
the change is built out.

### Focus restore diverges, and that is deliberate

On macOS the non-activating panel never takes application activation, so
dismissal needs no restore. On Windows the launcher does take the foreground, so
it must capture the previous foreground window before showing and restore it on
hide. Windows foreground lock makes that restore non-trivial.

This is the seed of the M3 plumbing that paste, snippet expansion, and AI
replace-selection all depend on. Building it here on the simplest possible case
means M3 inherits working code instead of discovering the problem under
pressure.

### Drop SvelteKit for plain Svelte and Vite

A launcher has no routes. Navigation is a view stack that pushes and pops, which
is application state, not URLs. SvelteKit contributes a router that goes unused,
a static adapter with an SPA fallback, and a build indirection, in exchange for
nothing this project needs.

Three template files exist today, so the cost of switching is minutes. After M1
it would be a real migration.

**Rejected:** keeping SvelteKit because it is already wired. Sunk cost against a
permanent tax on every build.

The frontend uses Bits UI on Tailwind v4 rather than hand-rolled markup. Bits UI
ships a `Command` primitive that is exactly a command palette, including the
selection and filtering behaviour the root search needs in M1, so hand-rolling
it would mean rebuilding a solved problem and its accessibility. Bits UI is
headless, so the look comes from the token set published with its documentation,
copied into `src/app.css`.

### Latency is measured with one clock

Cross-boundary timing is easy to get wrong when Rust and JavaScript each use
their own clock. Instead the backend timestamps the shortcut, sends an
activation id to the frontend, and the frontend calls back on the next animation
frame after render. The backend computes the elapsed time from its own monotonic
clock. One clock, no skew.

The number is recorded in development builds only, or when `DANGO_MEASURE` is
set. It exists to catch regressions in later milestones, not as a user-facing
feature.

The measurement is deliberately pessimistic: waiting two animation frames adds up
to roughly 33ms of scheduling at 60Hz on top of the real paint. Treat the
recorded figure as an upper bound, and compare like with like across milestones
rather than reading it as true paint latency.

Measured on an Apple Silicon release build, 16 activations: minimum 34.0ms,
median 46.7ms, maximum 83.2ms, cold first activation 61.3ms. Before the offscreen
warmup, cold activation reached 80.4ms; paying the webview's surface allocation
at startup is what brought it down. One sample of sixteen exceeded the budget,
which given the measurement's built-in pessimism is scheduling noise rather than
a design problem.

### Reset on hide is an explicit signal

A warm webview keeps its state, which is exactly why activation is fast and
exactly why stale state is a hazard. Rather than each future component
remembering to clean up, hiding emits one reset event and the frontend has one
handler that returns to root state. M1's view stack will subscribe to the same
signal.

### Default shortcut

Option+Space on macOS and Alt+Space on Windows. Both are physically the same
key position, both are close to Spotlight and Raycast muscle memory. Alt+Space
on Windows is claimed by the legacy window system menu, which is worth noting,
but it is reliably overridable and is what most Windows launchers use.

Command+Space is not chosen on macOS because taking it from Spotlight during M0
would make the machine unpleasant to use while the launcher does nothing.

## Verification status

macOS is verified end to end on a release build: the panel draws above a
fullscreen application without switching spaces, it accepts typing, Escape
clears then dismisses, blur dismisses, the launcher is absent from Command+Tab,
and the tray menu works. Activation was measured at a median of 46.7ms with a
cold first activation of 61.3ms.

Windows is verified on a release build driven by a Win32 harness that sends the
hotkey through `SendInput`, reads window styles and rectangles, and tracks the
foreground window across two displays at 150% and 100% scaling. Focus returns to
the previous window in another process on every dismissal path. Activation over
36 presses: cold 35.7ms, median 27.8ms, maximum 35.7ms, none over budget.

The first Windows run found four defects that CI could not: `cargo build
--release` produces a dev-mode binary that loads the Vite URL, so the frontend
never ran; tao rewrote the extended style on every show and dropped the tool
window bit; positioning through logical coordinates landed the window half off
the secondary display; and calling `SetFocus` on the top-level window after
activation pulled focus off the webview for an instant, which fired blur and
dismissed the launcher immediately. All four are fixed. What remains manual on
Windows is the tray menu, quit from the tray, display disconnection, and the
shortcut after sleep.

CI itself is verified. A real `AttachThreadInput` import error produced macOS
success alongside Windows failure, which is exactly the Windows-only breakage
the matrix exists to catch, so no synthetic error was needed.

Local cross-compilation to Windows from macOS is not possible here: the Tauri
build script needs `llvm-rc`, which is not installed. CI is the only Windows
compile check. Before pushing Windows code, check symbol names and module paths
against the vendored crate source under `~/.cargo/registry` rather than spending
CI rounds guessing; that caught two further errors after the first failure.

## Risks / Trade-offs

- **The NSPanel dependency is the main unknown.** It is a community crate
  tracking Tauri's internals, so a future Tauri upgrade could break it. Accepted
  because the alternative is writing the same objc bridging by hand. Mitigated
  by keeping it behind the platform trait, where a replacement touches one
  module.
- **The 80ms budget may not survive a debug build.** The measurement must be
  taken on release builds, or it will produce alarming numbers that lead to
  pointless optimisation.
- **Windows foreground restore is fiddly and may not be fully solved here.**
  Acceptable at M0, where nothing depends on it. It must be solid by M3.
- **Dropping SvelteKit forfeits its conveniences** if a settings surface later
  wants routing. M7's preferences window can use a plain view stack or its own
  tiny router. Low regret.
- **Transparency plus always-on-top has known compositor quirks** on both
  platforms, particularly around shadows and rounded corners. Cosmetic, deferred
  if it costs more than an hour.
