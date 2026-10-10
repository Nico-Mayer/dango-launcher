## Context

See proposal.md for the bug. The pieces involved:

- `WindowsAppIndexer::launch` in `src-tauri/src/platform/windows/apps.rs`
  pre-checks `app.target` with `is_file_path`, which only tests that the
  second byte is a colon. Explorer's target
  `::{52205FD8-5DFB-447D-801A-D0B52F2E83E1}` passes, `Path::exists` fails,
  and the indexer returns `LaunchError::NotFound`.
- `ApplicationsExtension::perform` in `src-tauri/src/extensions/applications/mod.rs`
  turns `NotFound` into `AppIndex::remove` plus the "no longer installed"
  message. It also returns that message when `AppIndex::get` misses.
- `AppIndex::rebuild` replaces the index from `AppIndexer::index`, which on
  Windows returns an empty list when enumeration errors part way
  (`enumerate().unwrap_or_default()`).
- `IndexingService` rebuilds on a fingerprint change and every 180 seconds,
  so a wrongly removed entry returns on its own and the bug repeats.

## Goals / Non-Goals

**Goals:**

- Shell namespace targets are never treated as files.
- Removal from the index is decided by the platform's enumeration, never by
  a launch-time guess.
- A refresh that cannot enumerate does not empty the index.

**Non-Goals:**

- Distinguishing launch failure causes on Windows.
- Changing the platform trait `AppIndexer`.

## Decisions

### Classify a path with the standard library

`is_file_path` becomes: the first component of `Path::new(s)` is a
`Component::Prefix`. On Windows the standard library's prefix parser accepts a
drive only as an ASCII letter followed by a colon, and also recognises UNC and
verbatim forms. `::{GUID}`, `steam://`, and an AppUserModelID have no prefix.

Rejected: tightening the hand-rolled check to "letter, colon, separator". It
works, but the standard library already encodes the Windows path grammar and
the project rule is off-the-shelf first.

### Only a file path is stored as a target

`IndexedApp::target` is documented as the filesystem path for reveal and
copy. The Windows indexer now applies the classifier once, during
enumeration, and stores `None` for anything else. The subtitle and the path
actions in the platform-neutral core follow from `target` being present, so
they need no platform knowledge, and the launch and reveal paths no longer
re-check the shape of the string. Launchability is still judged on the raw
target before it is dropped, so URL-scheme applications stay indexed.

Rejected: filtering in the core when building the result. The core would
need the Windows path grammar, and the macOS indexer never produces a
non-path target.

### The enumeration decides whether an entry is gone

`perform` handles any launch error the same way: rebuild the index, then
check whether the id is still present.

```
launch(app) --> Ok ------------------------------------> Done
           \-> Err(NotFound | Failed) --> index.rebuild()
                  \-> get(id) is None --> "That app is no longer installed..."
                  \-> get(id) is Some --> "Couldn't open <name>. Try again."
```

The platform's `NotFound` stays as a fast hint, but it no longer removes
anything by itself. `AppIndex::remove` and `Store::delete_app` lose their
only callers and are deleted.

A miss in `AppIndex::get` at the top of `perform` gets the same treatment:
rebuild once, look up again, and launch when found. This covers Enter on a
result rendered before a refresh replaced the index.

Rejected: keep the pre-check and only fix the classifier. That fixes Explorer
but leaves the next misclassified target to delete entries again.

Rejected: on launch failure, resolve `shell:AppsFolder\<id>` through
`SHCreateItemFromParsingName` to decide. Windows only, and a resolvable item
is not the same as a listed one. The enumeration is what fills the index, so
it is the only consistent authority on both platforms.

Rejected: map ShellExecute error codes 2 and 3 to `NotFound`. Not verifiable
locally for the AppsFolder case, and the rebuild makes it unnecessary.

### An empty enumeration keeps the previous index

`AppIndex::rebuild` ignores a result that is empty when the current index is
not, and logs it. An empty machine is not a real case; an empty result from
a non-empty machine is a failed enumeration. Without this guard, a failed
enumeration on the new launch path would report every application as gone.

### Cost

The rebuild runs on the failure path only, on the thread that already
performs the launch, never on the search path. Enumeration of about 135
shell entries takes well under a second on the author's machine. Two rebuilds
racing, one from the service and one from a launch, write the same snapshot
and are harmless.

### User-facing text

No string is added or changed. "That app is no longer installed. It's been
removed from results." and "Couldn't open {name}. Try again." stay as they
are.

## Risks / Trade-offs

- [A broken Start Menu shortcut whose executable was deleted stays listed by
  the shell] -> the entry stays and the launch reports "Couldn't open". The
  shell's own app list shows the same entry, so this matches what the user
  sees elsewhere.
- [Enumeration is slow on a machine with many applications] -> it only runs
  when a launch has already failed, and the message arrives after it.
- [The `is_file_path` change cannot be compiled on macOS] -> the tests live in
  the Windows module and run on the Windows CI job and on the author's
  Windows machine.
