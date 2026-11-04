## Why

"Toast" and "HUD" are Raycast's words, and neither says what the part is.
The v5 design work (`launcher-v5-split-shell`) set out to give Dango its own
look and language, so the two status surfaces get plain names.

## What Changes

- "Toast" becomes **status line**: the one-line status inside the launcher,
  in the ActionBar's left slot or at the pill's end while only the pill shows.
  Code: `StatusLine`, `status-line-*`, `ActionOutcome::Status`.
- "HUD" becomes **notice**: the small window shown after the launcher hides.
  Code: `Notice`, `NoticeWindow`, `notice.slint`.
- The tray's startup "notices" become **tray problems**, so "notice" means one
  thing. Code: `TrayStatus::problems`.
- The `hud` spec capability becomes `notice`. Every spec, the open
  `launcher-v5-split-shell` change, and AGENTS.md use the new words.
- **BREAKING** for the internal Rust API (`ActionOutcome::Toast`). No wire
  format, manifest field, config key, or user-visible text says "toast" or
  "HUD", so nothing outside the code changes.

## Non-goals

- No behaviour change. The status line and the notice look and act exactly as
  before.
- No rename of the protocol's `Status` (step and accessory states) or of the
  status dots. "Status line" is chosen so they can keep their names.
- No new or changed user-visible text.

## Platforms

Windows and macOS. The rename touches shared code and both platforms' notice
window code (`platform/windows`, `platform/macos`). Behaviour does not change,
so no hand checks: a build on each platform is enough. CI compiles Windows.

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
  `command-hotkeys`, `inline-answers`, `interface-copy`, and the deltas of
  `launcher-v5-split-shell`.
- Code: about 480 references in `src/` and `ui/`, mostly `src/ui/state.rs`,
  `src/ui/snapshot.rs`, tests, `src/slint_shell/`, `ui/shell/toast.slint`,
  `ui/shell/hud.slint`, `ui/globals.slint`.
- AGENTS.md platform traps that name the HUD.
- Depends on `launcher-v5-split-shell`: this branch stacks on `v5-closed-root`.
