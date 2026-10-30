## Context

See proposal.md - Why. The text exchange (`TextExchange`) is built once in
`lib.rs` setup, inside the branch that requires the database and the clipboard.
Its keys come from `platform::text_exchange`, which returns `None` when
`MacKeys::new` or `WindowsKeys::new` fails. Every consumer holds an
`Option<Arc<dyn TextTarget>>` and maps `None` to `TextError::PermissionMissing`.

Inside the exchange, every operation already checks `keys.permitted()` first and
returns `PermissionMissing` itself. On Windows `permitted()` is always true. So
the permission message is correct when it comes from inside the exchange, and
wrong when it comes from a missing exchange.

enigo 0.6.1 on macOS calls `has_permission` in `Enigo::new` and returns
`NewConError::NoPermission` when Accessibility is not granted
(`macos_impl.rs:513`). That is why the exchange is missing on a Mac started
without the permission.

## Goals / Non-Goals

**Goals:**

- A missing exchange and a missing permission are told apart in the message.
- The exchange exists whenever the clipboard can be opened and, on Windows, key
  injection can start.

**Non-Goals:**

- Watching the permission state or reacting to a grant beyond the next use.

## Decisions

### A new `TextError::Unavailable` variant

Consumers map a missing text target to it instead of `PermissionMissing`. The
message follows interface-copy: "Dango can't reach other apps right now. See
dango.log for the reason."

Alternative: reuse `TargetUnavailable(String)` with the message. Rejected,
because that variant means the target refused, a per-action condition, and
tests should be able to tell the two apart.

### Build the exchange outside the database branch

Open `CrateClipboard` once, before the store check. Build the exchange from it
with the history `Watcher` as `OwnWrites` when the history exists, and with the
existing `Unwatched` otherwise. The clipboard extension still needs both the
store and the clipboard.

Alternative: build a second exchange for the AI extension only. Rejected, two
exchanges would borrow the same clipboard without knowing about each other.

### Lazy enigo on macOS

`MacKeys::new` stops returning `Option`. It holds `Arc<Mutex<Option<Enigo>>>`,
empty at startup. `on_main` creates the `Enigo` on the main thread the first time
it runs and keeps it. Creation runs on the main thread because synthesis already
must, and startup created it there too. A creation failure maps to
`PermissionMissing` for `NoPermission` and to `TargetUnavailable` with
`KEYSTROKE_FAILED` otherwise, with the cause logged.

The exchange already checks `permitted()` before any keystroke, so in practice
creation only runs once the permission is there.

Alternative: rebuild the exchange when the permission changes. Rejected, it
needs a permission watcher and swapping a shared `Arc` under every consumer,
which is the next change's job at best and not needed for this one.

Windows stays eager. `WindowsKeys::new` does not depend on a permission, and a
failure there is a real startup problem the tray should show.

### Tray notice

When the exchange is `None`, push "Pasting is off, see dango.log" to the startup
notices, in the form the other notices use.

## Risks / Trade-offs

- [The first paste on macOS pays for enigo creation] -> Expected to be small
  next to the hide settle the paste already waits for. The verification task
  checks the first paste after launch by hand.
- [Windows code cannot be compiled locally] -> CI on `windows-latest` is the
  compile check, and verification runs on the Windows machine.
