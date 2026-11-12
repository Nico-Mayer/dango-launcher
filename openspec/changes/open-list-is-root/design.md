# Design

## Context

`ui::Launcher` gets the setting only through `Event::Reset { open_list }` and
`Launcher::starting(open_list)`, and stores it nowhere: `reset` sets
`root_open = open_list` and forgets it. `escape` and `query_changed` set
`root_open = false` whenever the prompt empties or the list is closed.

## Goals / Non-Goals

**Goals:** the reducer knows the root's resting state (open or closed) between
resets, so Escape and an emptied prompt land on it.

**Non-Goals:** changes to `Core`, the renderer, or the config reload path.

## Decisions

### 1. The reducer keeps the setting from the last reset

`Launcher` gets a field `open_root: bool`, set in `starting` and `reset` from
the value they already receive. Then:

- `escape`, prompt with text: clear it, set `root_open = open_root`.
- `escape`, empty prompt: if `root_open && !open_root`, close the list as
  today; otherwise `Effect::Dismiss`.
- `query_changed` to empty at root: `root_open = open_root`, and clear
  `picked` so the first row is selected.

A config change still applies at the next reset, as the spec already says,
because the value only enters at a reset.

Rejected: passing the config value with every key event, or letting `Core`
answer Escape. Both spread one rule outside the reducer, against the
architecture in AGENTS.md (all interaction rules in `src/ui/`).

Rejected: reading the current config on each Escape. A config reload mid
session would then change the Escape rule without a reset, which the spec
does not allow for this setting.

### 2. New change, not an edit of `smooth-launcher-open`

`smooth-launcher-open` is implemented and released, waiting only on a
Windows hand check before archive. Changing its specs now would mix a new
behaviour into a done change. This change's MODIFIED deltas copy text that
`smooth-launcher-open` adds, so archive order is fixed: that change first.

### 3. Settings reference text

The doc comment on `config::Launcher::open_list` (source of the schema and
settings reference) changes. Written with the ux-writing skill and checked
against `openspec/specs/interface-copy`.

- Old: "Open the list of suggestions each time the launcher starts fresh, as
  if you pressed Down. Default false."
- New: "Show the list of suggestions whenever the prompt is empty, not only
  after you press Down. Escape then hides the launcher. Default false."

## Risks / Trade-offs

- [With the setting on, the pill alone or the "Needs attention" card alone can
  no longer be reached] → Accepted, it is what the setting asks for. Pinned
  items still show first in the open list.
