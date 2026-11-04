## Context

See proposal.md for the why. "Toast" and "HUD" appear about 480 times in
`src/` and `ui/` and about 170 times in `openspec/specs/`, plus in the open
`launcher-v5-split-shell` change, AGENTS.md and one comment in
`tools/dango-ui-mcp`. None of them reaches a wire format, manifest, config
key, accessible label, window title, or user-visible string; the only string
is a log line (`could not show the HUD`).

Every token in the repo that contains `toast` or `hud` is one of these names
(checked with `rg -o -i "[a-z_]*(toast|hud)[a-z_]*"`): no English word or
foreign identifier contains them.

## Goals / Non-Goals

**Goals:**

- One word per part everywhere: specs, code, comments, tests, AGENTS.md.
- The build and every test pass after each commit.

**Non-Goals:**

- Any behaviour or look change.
- Renaming the protocol's `Status`, the status dots, or the existing
  "status lines".

## Decisions

### Names, checked for collisions

| Old | New |
|---|---|
| toast | ticker |
| HUD | notice |
| tray notice | tray problem |
| spec capability `hud` | `notice` |

Before choosing, each candidate was counted in `openspec/specs`, `src` and
`ui`. "Ticker" and "notice" (once the tray's notices move) have no other use.

Rejected:

- **Status.** The protocol has `Status` for step and accessory states, which
  the status dots draw.
- **Status line.** Already names the tray's lines ("Config file loaded"), the
  text beside a pinned result, and a task's progress line, and titles the
  interface-copy requirement "Status lines say what is happening".
- **Keeping "tray notices".** Two different "notices" would bring back the
  confusion this change removes.

### Case-aware substring rules, not a name table

Most hits are compound names (`ExpireToast`, `SlintToastStyle`,
`set_toast_shown`, `hud_origin`, `MacHudWindow`, `Theme.hud-width`, test
names). Word-boundary regexes miss them; a per-name table would be long and
incomplete. Since every token containing the old words is ours, plain
substring rules cover all of them in one pass:

- `Toast` → `Ticker`, `toast` → `ticker`, `TOAST` → `TICKER`
- `Hud` → `Notice`, `hud` → `notice`, `HUD` → `NOTICE` in code; "HUD" →
  "notice" in prose

Slint kebab names follow (`toast-shown` → `ticker-shown`), and so do the
setters Slint generates (`set_ticker_shown`). Files move with `git mv`. A
prose pass then fixes articles and capitals ("a HUD" → "a notice", "The HUD"
→ "The notice").

The tray's `notices` are renamed first, in their own commit, so `notice`
never means two things in the tree.

### Spec wording edited in place

The rename changes no requirement, so the change sets `skip_specs` and edits
the main specs and the open change's files directly. Requirement titles
change with them, for example "A toast reports status in the ActionBar"
becomes "A ticker reports status in the ActionBar". The `hud` folder moves to
`notice` with its H1, and the "see `hud`" links follow.

The `launcher-v5-split-shell` deltas MODIFY requirements by title, and
`openspec validate` does not check that the title exists in the main spec;
only `archive` does. So main specs and the v5 change are reworded in one
commit, and that commit checks every MODIFIED title against the main spec.

### Archive order

Archive `launcher-v5-split-shell` at the top of the stack, after this branch.
Archiving it on a lower branch would edit the same specs this branch edits,
and restacking would conflict.

### Contracts

No change to the extension manifest or the wire format of the view protocol.
`ActionOutcome` is a Rust enum for built-ins, not part of the protocol.

### Text

No user-visible string changes. The log line becomes "could not show the
notice".

## Risks / Trade-offs

- [A substring rule hits something it should not] → The token list above
  shows none; the commit's check greps for leftovers and the diff is reviewed
  for prose.
- [macOS code does not compile] → Only Dango's own names change there
  (`MacHudWindow`, `HUD_WINDOW_LEVEL`); CI's macOS job confirms after push.

## Migration Plan

None. Alpha, nothing stored or configured uses the names.
