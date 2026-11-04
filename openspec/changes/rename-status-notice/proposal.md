## Why

"Toast" and "HUD" are Raycast's words, and neither says what the part is.
The v5 design work (`launcher-v5-split-shell`) set out to give Dango its own
look and language, so the two status surfaces get plain names.

## What Changes

- "Toast" becomes **ticker**: the one-line status inside the launcher, in the
  ActionBar's left slot or at the pill's end while only the pill shows. Code:
  `Ticker`, `TickerStyle`, `ActionOutcome::Ticker`, Slint `Shell.ticker-*`.
- "HUD" becomes **notice**: the small window shown after the launcher hides.
  Code: `Notice`, `NoticeWindow`, `notice.slint`.
- The tray's startup "notices" become **tray problems** first, so "notice"
  means one thing. Code: `TrayStatus::problems`.
- The `hud` spec capability becomes `notice`. Every spec, the open
  `launcher-v5-split-shell` change, AGENTS.md and the `dango-ui` MCP proxy
  use the new words.
- **BREAKING** for the internal Rust API (`ActionOutcome::Toast`). No wire
  format, manifest field, config key, accessible label, or user-visible text
  says "toast" or "HUD", so nothing outside the code changes.

## Non-goals

- No behaviour change. The ticker and the notice look and act exactly as
  before.
- No rename of the protocol's `Status`, the status dots, or the existing
  "status lines" (tray, pinned results, tasks).
- No new or changed user-visible text.
- CHANGELOG and `.design/` keep their history as written.

## Platforms

Windows and macOS. The rename touches shared code and both platforms' notice
window code (`platform/windows`, `platform/macos`). Behaviour does not change,
so no hand checks: this machine builds Windows, CI's macOS job builds macOS.

## Capabilities

### New Capabilities

None. The `hud` capability moves to `notice` with the same requirements,
reworded; that is a rename, not a new behaviour.

### Modified Capabilities

None at the behaviour level. Specs change wording only, so the change sets
`skip_specs` and edits the spec text directly.

## Impact

- Specs: `design-system`, `hud` (moves to `notice`), `view-protocol`,
  `launcher-shell`, `background-tasks`, `ai-commands`, `app-updates`,
  `command-hotkeys`, `inline-answers`, `interface-copy`, `global-activation`,
  and all files of `openspec/changes/launcher-v5-split-shell`.
- Code: about 480 references in `src/` and `ui/`, one comment in
  `tools/dango-ui-mcp/src/main.rs`. Files: `ui/shell/toast.slint`,
  `ui/shell/hud.slint`, `src/slint_shell/render/hud.rs`.
- AGENTS.md platform traps that name the HUD.
- Builds on `launcher-v5-split-shell`, now archived: this branch stacks on
  `v5-closed-root`. The archive in `openspec/changes/archive/` keeps the old
  words as history.
