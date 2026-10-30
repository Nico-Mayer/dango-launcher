## Context

See proposal.md - Why. Today:

- `platform/macos/mod.rs` has `accessibility_trusted()` and
  `prompt_for_accessibility()`. `TextExchange::request_permission` wraps the
  prompt and has no caller.
- Keyword expansion is a `Service` of the snippets extension. Its `start` calls
  `platform::start_key_monitor`, and on macOS a refused `CGEventTap` returns
  `None`, so `start` fails and the host records an activation error. Nothing
  retries.
- The hyperkey is applied by `apply_hyperkey` in `lib.rs` at startup and on a
  config reload. A refused tap gives the generic "could not start; unavailable
  on this platform or refused by the OS".
- The tray menu is built once in setup. Startup notices are fixed; only the
  config status line has a live handle (`ConfigStatusItem`).
- Root providers are async and run per query, off the main thread. The ranker
  sorts an empty query by frecency and marks the top few `suggested`.
- The extension host lives in app state as `Arc<Mutex<ExtensionHost>>`.

## Goals / Non-Goals

**Goals:**

- One source of truth for the permission state, cheap enough to ask per query.

**Non-Goals:**

- Watching TCC for changes. Polling is enough at this scale.
- Restarting Dango after an Input Monitoring grant. macOS offers that itself.
- Any change to how paste, selection, or window management check the permission
  inside an operation.

## Decisions

### Hand-rolled checks over `tauri-plugin-macos-permissions`

Evaluated `tauri-plugin-macos-permissions` 2.3.0. It exposes
`check_accessibility_permission`, `request_accessibility_permission`,
`check_input_monitoring_permission` (`IOHIDCheckAccess`), and
`request_input_monitoring_permission` as public async Rust functions. It falls
short in two ways:

- `request_input_monitoring_permission` only opens the settings pane. It never
  calls `CGRequestListenEventAccess`, so Dango is not added to the Input
  Monitoring list and the user would have to add the app by hand with "+".
- `request_accessibility_permission` shows the prompt but does not open the
  pane, so it covers only half of "Grant access" either way.

Everything it does well is one call on crates Dango already depends on:
`AXIsProcessTrusted` and `AXIsProcessTrustedWithOptions` (already used,
`objc2-application-services`), `CGPreflightListenEventAccess` and
`CGRequestListenEventAccess` (`objc2-core-graphics`, `CGEvent` feature already
on). Opening the pane uses `tauri-plugin-opener`, already in use. Adding the
plugin would bring `macos-accessibility-client` and an async wrapper around four
synchronous calls, and still need the two missing pieces.

### A `Permissions` trait in `platform`

```
enum Permission { Accessibility, InputMonitoring }

trait Permissions: Send + Sync {
    fn missing(&self) -> Vec<Permission>;   // never prompts
    fn request(&self, permission: Permission); // prompt or register, never blocks
}
fn settings_url(permission) -> &'static str
```

macOS: `missing` reads `AXIsProcessTrusted` and `CGPreflightListenEventAccess`.
`request` calls the Accessibility prompt or `CGRequestListenEventAccess`. The
URLs are `x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility`
and `...?Privacy_ListenEvent`. Windows: `missing` is always empty and `request`
does nothing. This follows the narrow-trait rule; no `cfg` in feature code.

### A built-in `permissions` extension with a root provider

The warning items come from a root provider, so they are computed per query,
off the main thread, from `Permissions::missing()`. That makes "re-check on
show" free: the activation query already asks. Nothing is added to the `show`
path, which keeps the 80ms budget untouched.

Each item has `icon:triangle-alert`, a title such as "Accessibility is off", a
subtitle naming the features, keywords including the permission's name and
"permission", and the primary action "Grant access". The action calls
`request`, opens the settings URL through the opener, and hides the launcher.

Alternative: a banner in the launcher frame. Rejected, because a banner is not
an item the user can search, act on with Enter, or reach by keyboard through the
`Command` primitive, and it would need new frontend state.

Alternative: put the provider in the system extension. Rejected, the system
extension is a set of commands. A separate extension keeps one toggle for the
warnings.

### Pinned candidates on an empty query

`Candidate` gains `pinned: bool`. On an empty query the ranker puts pinned
candidates first, in provider order, before the frecency list, and they do not
count toward `SUGGESTION_LIMIT`. On a typed query `pinned` has no effect; the
items rank by match like any other. Only the permissions provider sets it.

### One poller for the tray

A background thread asks `Permissions::missing()` every 2 seconds for as long as
Dango runs. It runs on Windows too, where the set is always empty and nothing
ever changes; that costs less than a platform branch in feature code. It keeps
the last set, and on a change updates the tray on the main thread.

The two tray items ("Grant Accessibility", "Grant Input Monitoring") are created
once at startup and kept in app state with the menu. A change removes both and
inserts the missing ones at the top, Accessibility first. The menu event handler
maps `grant-accessibility` and `grant-input-monitoring` to the same
request-and-open as the root action. This replaced rebuilding the whole menu
during implementation: the startup notices stay where they are and need no new
state.

### No restart after a grant

The first plan restarted the hyperkey and keyword expansion when the poller saw
a grant. Verification showed it can never help: macOS applies an Input
Monitoring grant only after Dango restarts, and offers "Quit & Reopen" for it,
while both features need only Input Monitoring. An Accessibility grant already
reaches paste without a restart, because key injection is created on first use.
The restart path was removed, and the features start on the restart macOS asks
for, through the normal startup path.

### Failure messages that name the permission

`apply_hyperkey` asks `Permissions::missing()` when the start fails. If Input
Monitoring is missing, the message is "hyperkey: needs the Input Monitoring
permission". `MacKeyMonitor::start` logs the same for keyword expansion.

## Risks / Trade-offs

- [`CGPreflightListenEventAccess` may not match what the tap needs] -> The
  author's Mac shows the tap refused with Accessibility granted, which points to
  Input Monitoring. The verification tasks check that granting it starts both
  the hyperkey and keyword expansion; if not, the check is wrong and the change
  stops there.
- [A permission granted to a debug build is a different TCC entry from the
  installed app] -> Verification runs against a bundled build started with
  `open`, as in `fix-text-exchange-availability`.
- [`CGRequestListenEventAccess` may show a system prompt besides opening the
  pane] -> Acceptable: it only happens from the user's own action. Checked by
  hand in verification.
- [Changing the tray menu while it is open] -> It changes only when the missing
  set does, which happens when the user acts in System Settings, not while the
  menu is open.

## Open Questions

None. Verification showed the hyperkey's active tap starts with Input
Monitoring alone, recorded in the known platform traps.
