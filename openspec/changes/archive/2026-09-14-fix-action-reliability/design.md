## Context

See proposal.md - Why. The three defects are unrelated in cause and share one
symptom: nothing happened and nothing said why. What follows is the reasoning
behind the three fixes, all of which are already on `main`.

The surfaces involved: the render event `dango://render` (host to webview, not
part of the extension-facing protocol), `App.svelte`'s view stack, and the
clipboard watcher's own-write queue.

## Goals / Non-Goals

**Goals:**

- One way for the view stack to learn who owns a view, whatever started it.
- A failure that is always readable by the user who caused it.
- Own-write suppression that survives the system re-encoding what it was given.

**Non-Goals:**

- Changing the extension-facing view protocol or its version.
- Reporting failures anywhere other than the launcher.

## Decisions

### The owner travels in the render envelope, not in the tree

`run_command` resolves the owning extension once and emits
`RenderPayload { owner, tree }`. The frontend sets `viewOwner` from the event,
which makes the event the single source of truth, so `invoke_command` no longer
returns the owner and `confirm()` no longer records one.

The owner is a host concern - the registry knows it, the extension does not
declare it - so putting it in the envelope keeps `ViewTree` exactly as
extensions produce it and leaves `protocolVersion` at 1.

Alternatives rejected:

- Add the owner to `ViewTree`. It is a versioned, extension-facing contract, and
  this is not something an extension should be able to state about itself.
- Have the frontend ask the backend who is running when a view arrives without
  an owner. A round trip to recover something the sender already knew.
- Let `run_action` resolve the owner from a running-command slot when the
  frontend sends none. It hides the coupling and breaks as soon as two commands
  can be in flight.

### A failure outlives the reset, and the user clears it

`resetToRoot` no longer clears `failure`. The paste path hides the launcher
before it does the work, so its failure arrives after the hide, and whether it
arrives before or after the reset event is a race the frontend cannot win: both
orders were observed. Clearing on the user's next action instead of on hide
makes the message reliably readable exactly once.

Alternatives rejected:

- Order the backend's hide and its response so the report always lands after the
  reset. It makes correctness depend on event ordering across an IPC boundary
  and on the main-thread hop `hide_after_launch` uses.
- Show failures in a notification instead. A whole surface, plus a permission,
  for a message that belongs in the window the user is about to reopen.

### An image write is recognised by size and recency

The pasteboard re-encodes: a 1571 byte favicon comes back as 5427 bytes, stably
so, which is why the second round trip matched and the first never did. Exact
byte comparison therefore cannot recognise an image Dango wrote.

A pending write now carries the image's decoded dimensions and the moment it was
made. Text still matches exactly. An image matches when the dimensions match and
the write is younger than two seconds, which covers the crate's 500ms poll and
the copying application finishing.

Alternatives rejected:

- Compare decoded pixels. Exact, but it decodes two images on every clipboard
  change, and a screenshot is megabytes.
- Drop the time bound and match on dimensions alone. A stale expectation would
  then swallow the user copying the same image later, which the surrounding rule
  explicitly protects.
- Read the clipboard back after writing and expect those bytes too. The read
  races the change notification the expectation is meant to cover.

## Risks / Trade-offs

- A user copying a different image of identical dimensions within two seconds of
  Dango writing one loses that entry → the same asymmetry the exclusion checks
  already accept, and the window is bounded by the crate's poll interval.
- A carried failure could confuse if it is read as describing the current
  activation → it is cleared by the first keystroke, so it survives only until
  the user does anything.
- The render envelope now differs from the tree the extension produced, so a
  reader of the event has one more layer → it is one struct, named for what it
  carries.

## Migration Plan

None. No stored state, no protocol version, no data to migrate.
