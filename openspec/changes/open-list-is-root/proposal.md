# Proposal

## Why

With `launcher.open-list` on, the open empty-query list is the root view the
launcher starts with. Escape still treats it as an extra stage: the first
Escape only closes the list, so dismissing takes two presses, and the closed
pill it leaves is a state the user never asked for.

## What Changes

- With `launcher.open-list` on, the open list is the resting root view. Escape
  on the empty prompt hides the launcher at once (a deliberate hide), with no
  "close the list" stage.
- With the setting on, clearing the prompt (Escape with text, or deleting the
  last character) returns to the open list, not to the closed pill.
- With the setting off, nothing changes: Arrow Down opens the list, Escape
  closes it, a second Escape hides.
- The settings reference text for `launcher.open-list` describes the new
  behaviour.

## Non-goals

- No way to close the list while the setting is on. Users who want the pill
  turn the setting off.
- No change to pushed views, the action panel, or the hold after an
  interrupting hide.
- No change to what the open list contains.

## Platforms

Windows and macOS alike. The change lives in the platform-neutral launcher
state machine; no platform code is touched.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `launcher-shell`: "Dismissal" gets the rule that, with the setting on,
  Escape on the empty prompt hides. "The open list at start is a config
  setting" replaces its "Escape stages still apply" scenario.
- `root-search`: "Empty query behaviour" says an emptied query returns to the
  open list when the setting is on.

## Impact

- `src/ui/state.rs` (`escape`, `query_changed`, `reset`, `starting`) and
  `src/ui/tests.rs`.
- `src/config/mod.rs` doc comment for `open_list`, regenerated schema and
  settings reference.
- Depends on the `smooth-launcher-open` change, which adds the setting and is
  not yet archived (open Windows hand check). Its deltas must be archived
  before this change's.
