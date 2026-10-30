## Why

On macOS, Dango needs two privacy permissions, and today nothing tells the user
when either is missing until a feature fails. Accessibility gates pasting,
reading the selection, and moving windows. Input Monitoring gates the keyboard
event taps behind keyword expansion and the hyperkey. With Accessibility granted
and Input Monitoring missing, the hyperkey has never started on the author's Mac,
and the only trace is a line in `dango.log`. Keyword expansion fails the same
way with only a message on stderr.

The selection-and-paste and window-management specs already promise that the
launcher "offers to request" the permission. Nothing does that today:
`request_permission` exists but has no caller.

## What Changes

- Dango tracks whether each macOS permission is granted, re-checks when the
  launcher is shown, and polls every 2 seconds while it runs.
- Each missing permission shows as a warning item at the top of root search on
  an empty query, naming the permission and the features it turns off. Its
  primary action, "Grant access", asks the system for the permission and opens
  the matching pane in System Settings. The item is also found by searching.
- Each missing permission shows as a tray item that opens the same pane.
- When a missing permission is granted, the warning item and tray item
  disappear. Accessibility takes effect at once. Input Monitoring takes effect
  after Dango restarts, which macOS requires and offers to do.
- A failed keyword expansion or hyperkey start on macOS names the missing
  permission instead of a generic "refused by the OS".
- Windows shows no permission warnings. It has no equivalent permission.

Milestone: M7 polish, the "permission onboarding" item, scoped down to warnings.
Platforms: macOS has all the behaviour. Windows is covered by stating that it
shows nothing, because no Windows permission gates these features. The
elevated-window limit stays as it is.

## Non-goals

- A first-run or guided onboarding view, and a stored "onboarding seen" flag.
- Screen Recording, Full Disk Access, Automation, or any other permission.
- Reacting to a permission revoked while Dango runs, beyond showing the warning
  again.
- Starting keyword expansion or the hyperkey live after a grant. macOS applies
  an Input Monitoring grant only after Dango restarts.
- Changing the inline failure messages from paste, selection, and window
  management. They keep naming the permission.
- Running Dango elevated on Windows, or warning about elevated windows.

## Capabilities

### New Capabilities

- `permissions`: the macOS permission state, how it is re-checked, the root
  warning item and its grant action, the tray items, when a grant takes effect,
  and Windows showing nothing.

### Modified Capabilities

- `keyword-expansion`: "Observation follows the extension's enabled state"
  changes its macOS permission scenario to name Input Monitoring and adds one for
  a grant followed by the restart macOS requires.
- `hyperkey`: "The hyperkey applies at startup and live on a config change"
  gains scenarios for a missing permission on macOS and for a grant followed by
  the restart macOS requires.

## Impact

- `src-tauri/src/platform/`: a `Permissions` trait with a macOS implementation
  (`AXIsProcessTrusted`, `CGPreflightListenEventAccess`,
  `CGRequestListenEventAccess`, the existing Accessibility prompt) and an empty
  Windows one.
- A new built-in extension with a root provider for the warning items and the
  grant action.
- `src-tauri/src/search/`: a way for a candidate to stay on top of an empty
  query.
- `src-tauri/src/lib.rs`: the poll and tray items with a live handle.
- `src-tauri/src/platform/macos/hyperkey.rs`, `keymonitor.rs`: failure messages
  that name the permission.
- No change to the config file, the view protocol, or stored data.
