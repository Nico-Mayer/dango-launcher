## Why

The view protocol promises two things it cannot deliver. A list can declare
that its command filters, but nothing ever sends the query to the command, so
such a list never narrows. And an action can declare a shortcut such as Shift+1
or Alt+/, but key events carry the character the key produced (`!`, or a
composed character), so the shortcut never fires. Letters and named keys such
as Enter or the arrows are not affected: they keep their name with modifiers. Both are part of a versioned
contract that later third-party extensions will read, so they should say only
what Dango does.

## What Changes

- **BREAKING** Remove the filtering choice from list and grid views. The
  launcher always filters a list or grid against the query. A command that
  needs to filter itself can bring the choice back with an effect that
  delivers the query.
- Drop the scenario "Command owns filtering" and the "command filters" parts of
  the item-limit requirement.
- **BREAKING** Document that an action shortcut with Shift or Alt names a letter
  key or a named key (Enter, Tab, Escape, the arrows, and the other keys the
  key handling names). Dango refuses a view tree that declares any other key
  with Shift or Alt, and shows the same error as for any other invalid tree.
- State the macOS gap: Option with a letter and without Cmd or Ctrl types a
  composed character, so such a shortcut does not fire there.
- The protocol version stays at 2.

## Non-goals

- Command-side filtering, or any effect that sends a view's query to its
  command.
- Matching shortcuts by physical key, or making Shift or Alt work with digits
  and symbols.
- Fixing the macOS Option gap or the German AltGr letters on Windows.
- Checking shortcuts on root search results. Only built-ins produce those, and
  none uses Shift or Alt.
- Any change to how a refused tree looks or reads.

## Platforms

Windows and macOS. The change is platform-neutral: the filtering removal and
the tree check run in the shared state machine. The macOS Option gap already
exists and is only stated.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `view-protocol`: remove declared filtering ownership; the item limit no
  longer mentions command filtering; add a requirement on which keys a Shift or
  Alt shortcut may name.

## Impact

- `src/protocol.rs`: `Filtering` and the `filtering` fields of `ListView` and
  `GridView` go away; `Shortcut` documents the shortcut key rule.
- `src/ui/state.rs`: the tree check refuses a bad shortcut; `group` loses its
  filtering argument.
- Built-ins that set `filtering` (clipboard, snippets, system, the
  invocation test helper) and the tests that use `Filtering`.
- No config, manifest, or stored data change.
