## Context

See proposal.md for why. The current code:

- `Filtering` (`src/protocol.rs`) is a required field on `ListView` and has a
  default on `GridView`. `group` in `src/ui/state.rs` skips launcher filtering
  for `Command`, and `query_changed` only stores the text on a pushed view. No
  effect carries the query to the command. Every built-in sets `Launcher`.
- A shortcut matches through `Chord::from(&Shortcut)` (`src/ui/input.rs`),
  which uses the declared key. The key event's chord (`src/slint_shell/keys.rs`)
  uses the text Slint reports, lowercased. Slint's winit backend takes that text
  from winit's logical key, so Shift+1 arrives as `!` (and differs on a German
  layout). On macOS, winit keeps Option's effect when neither Cmd nor Ctrl is
  held, so Option+P arrives as `π`; with Cmd or Ctrl held it reports `p`.
- `Launcher::rendered` refuses a tree with an unknown version
  (`ViewError::Unsupported`) or grid columns out of range (`ViewError::Invalid`,
  "This view can't be shown"). Trees that come back as an action outcome
  (`Replaced`, `Removed`) are pushed without that check.

## Goals / Non-Goals

**Goals:**

- The protocol types and the spec describe only what Dango does.
- One check refuses a bad tree, whichever way it arrives.

**Non-Goals:**

- Physical-key matching, or any change to the renderer's key handling.

## Decisions

### Remove the filtering field, not just the variant

`Filtering` with one variant would be a field that cannot vary. Remove the
enum and the `filtering` fields of `ListView` and `GridView`; `group` always
filters. Serde ignores unknown fields here, so a tree that still sends
`filtering` parses.

Alternative: add an effect that re-invokes the command with the query. Rejected
for now: no command needs it, and it would need cancellation and ordering rules
for each keystroke that nobody would exercise.

### Refuse a bad shortcut as an invalid tree

A Shift or Alt shortcut whose key is not one ASCII letter, compared case
blind, makes the tree invalid. The check covers every action in the tree: the
view's actions, each list or grid item's actions, and the empty state's
actions. It reuses `ViewError::Invalid`, so the user sees the same "This view
can't be shown" error as for grid columns out of range. No new text.

Move the version, column, and shortcut checks into one function and call it
wherever a tree enters the stack: `rendered` and the `Replaced` and `Removed`
action outcomes. Otherwise a tree returned by an action would skip the rule.

Alternatives:

- Drop the bad action and keep the view. Rejected: a silent drop hides the
  mistake from the extension author, and the spec already refuses invalid
  trees whole.
- Refuse at deserialisation. Rejected: built-ins build trees in Rust and never
  pass through serde.
- Translate symbols back to their unshifted key. Rejected: the mapping depends
  on the layout (English and German differ), and the renderer does not see the
  physical key.

### Protocol version stays at 2

Both parts are breaking: a field leaves and some trees that were accepted are
now refused. The version stays at 2 because every tree producer today is a
built-in compiled against the same types, so no producer can send a tree from
before this change, and nobody ever sent `Command` or a Shift or Alt symbol
shortcut. A bump would only change the constant, the "View kinds" requirement,
and the tests that pin version 1. Project stage allows breaking changes without
shims. The code comment on `PROTOCOL_VERSION` says it is bumped when the shape
changes; reword it to say it is bumped when a tree from outside this build
could be read wrongly, which is not yet possible.

## Risks / Trade-offs

- [On macOS, Alt with a letter and without Cmd or Ctrl still never fires] →
  The spec states it. The check could refuse Alt without Cmd or Ctrl, but that
  would also refuse Alt+letter on Windows, where it works.
- [On a German Windows layout, Ctrl+Alt acts as AltGr, so Ctrl+Alt+Q, E, or M
  reports `@`, `€`, or `µ` and does not match] → Accepted. No built-in uses
  Ctrl+Alt; the spec covers letter keys in general and does not promise these.
- [A third-party tree from an older build sends `command` filtering] → None
  exists. The field is ignored and the launcher filters.
