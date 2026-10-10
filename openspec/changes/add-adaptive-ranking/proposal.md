## Why

Ranking knows what the user launches but not what they typed to get there, so
a short query keeps landing on the item with the best text match and frecency
even after the user has corrected it to something else several times. Every
launcher the author compares Dango against learns from that correction. This
is the personalisation gap in daily use.

## What Changes

- Dango learns pairs of typed query and chosen item when a root result is
  confirmed, and uses them in ranking: an item picked before for exactly the
  typed string outranks every text match, and an item picked for a longer
  string that starts with the typed one gets a boost. Typing past a learned
  string switches its influence off.
- Learned pairs decay with the same half life as frecency, persist across
  restarts in a new table, and are kept in memory so the search path never
  reads the database.
- The frontend sends the current query along with a root confirm so the pair
  can be recorded. Hotkey launches and actions inside a pushed view teach
  nothing.
- Any successful root action teaches, the same rule frecency uses for counting
  use.

Milestone: a follow-up to M1 core, where the ranking pipeline and frecency were
built. Platforms: Windows and macOS, no platform-specific code.

## Non-goals

- Context signals such as the previous application or time of day.
- User-defined keywords or aliases on applications and root items. That is a
  config-shape change, not a ranking change.
- Changing the fuzzy scorer, the frecency formula, the suggestions section on
  the empty query, or the alias tier.
- Any interface for inspecting or clearing what was learned. Deleting the
  table rows is the reset, and the roadmap's sync discipline keeps the door
  open for more later.
- Sync of learned pairs between machines. The table follows the syncable
  record convention and nothing more.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `root-search`: "Ranking combines match quality and frecency" gains the
  learned-input tier and boost; "Launching an item updates its frecency"
  becomes recording both frecency and the query the item was chosen for. A new
  requirement in the same capability covers what a learned input is, when it
  fires, how it decays, and how a correction overrides it.

## Impact

- `src-tauri/src/store/migrations.rs`: one new table, `input_history`.
- `src-tauri/src/store/mod.rs`: load and save for it.
- `src-tauri/src/ranking/`: a new in-memory table beside `FrecencyTable`, a new
  tier in `DangoRanker`, and the prefix boost.
- `src-tauri/src/lib.rs`: `run_action` and `invoke_command` take an optional
  query and record the pair on success.
- `src/App.svelte`: the two root confirm paths pass the query.
- No change to the extension manifest, the view protocol, or the config file.
