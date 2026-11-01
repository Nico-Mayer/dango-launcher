## Context

An audit read every spec in `openspec/specs/` against `src/` and `ui/` after
the Slint switch. This design records which side wins where spec and code
disagree. The proposal lists what changes per capability.

## Goals / Non-Goals

**Goals:**

- Every requirement describes what the code does today, or is a tracked bug.
- No WebView-era wording, no implementation detail, no rule stated twice.
- Every requirement passes the `openspec/config.yaml` spec rules.

**Non-Goals:**

- Code changes. Restructuring the capability list.

## Decisions

### The spec follows the code by default

Most disagreements are cases where the code made a deliberate, later choice
(the hold after dismiss, pinned items, CapsLock-only hyperkey, preferences in
the config file) and the spec was not updated. Changing the spec is right for
those.

The exceptions are seven cases where the spec states the intended behaviour
and the code is wrong. Their spec rule stays, and the bug is an issue:

| Issue | Spec | Rule kept |
| --- | --- | --- |
| #16 | selection-and-paste | Empty selection on macOS: no keystroke |
| #17 | selection-and-paste | Direct selection read is bounded on macOS |
| #18 | ai-commands | A broken prompt is not offered |
| #19 | global-activation, interface-copy | Notice names the configured chord |
| #20 | ai-providers | No credential to a local provider |
| #21 | keyword-expansion | Expansion recovers on Windows |
| #22 | root-search | A typed query shows no headings and does not put pinned items first |

Alternative rejected: weaken those rules to match the bugs. That would hide
the bugs and lose the intent.

### The hold is the single owner of hide behaviour

view-protocol and ai-commands each said hiding resets state. launcher-shell's
hold now decides that. The other specs say "when the launcher session ends
(see launcher-shell)" instead of "when the launcher hides".

### One copy per rule

Where two specs state the same rule, the narrower capability keeps it and the
other links:

- Release of the global shortcut on exit: global-activation.
- A disabled command's hotkey: command-hotkeys.
- Dango's own clipboard writes: selection-and-paste.
- Requesting a permission and noticing a grant: permissions.
- Activation speed: launcher-shell, qualitative only.
- Theme structure (every value from `Theme`): AGENTS.md, not a spec.

### Config write-back is cut to file creation

Nothing writes the config file except creating a missing one. The write-back
rules are reduced to that case. A future settings editor brings its own
rules with it.

Alternative rejected: keep the rules as a future contract. Specs describe
the app as it is, and a contract with no caller cannot be tested.

### Reasons kept out of the spec text

Two platform gaps are stated in the specs without their cause, which is
implementation:

- macOS keeps a fixed launcher height because resizing there moves the top
  edge, and keeping it in place is not built (AGENTS.md platform traps).
- On macOS, CapsLock does nothing where the hyperkey cannot see input, because
  it is remapped to F18 below the level Dango observes.

### Open points, left as they are

- local-store's syncable-record convention versus records-storage calling
  usage ranking machine-local. Needs a decision about sync, not a text fix.
- root-search credits a command when it starts, not when it succeeds. Not
  clear which is intended.
- release-pipeline's "nothing to release" message is unverified. Needs a dry
  run, not a text fix.

## Risks / Trade-offs

- [Deltas restate long requirements] -> MODIFIED blocks copy the full
  requirement, so archive does not lose scenarios. `openspec validate
  --strict` checks the headers match.
- [An audit claim is wrong] -> Each delta is checked against the cited code
  before it is written.
