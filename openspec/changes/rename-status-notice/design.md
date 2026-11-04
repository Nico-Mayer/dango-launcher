## Context

See proposal.md for the why. "Toast" and "HUD" appear about 480 times in
`src/` and `ui/` and about 170 times in `openspec/specs/`, plus in the open
`launcher-v5-split-shell` change and AGENTS.md. None of them reaches a wire
format, manifest, config key, or user-visible string, so the rename is
internal. Two existing names stand in the way of the plain choices: the
protocol's `Status` (step and accessory states) and the tray's startup
`notices`.

## Goals / Non-Goals

**Goals:**

- One word per part everywhere: specs, code, comments, tests, AGENTS.md.
- The build and every test pass after each commit.

**Non-Goals:**

- Any behaviour or look change.
- Renaming the protocol's `Status` or the status dots.

## Decisions

### Names

| Old | New (prose) | New (code) |
|---|---|---|
| toast | status line | `StatusLine`, `status_line`, Slint `status-*` on `Shell` |
| `ToastStyle` | style | `StatusStyle` (Rust and Slint) |
| `ToastAction` | action | `StatusAction` |
| `HeldToast` | | `HeldStatus` |
| `Event::ToastExpired` | | `Event::StatusExpired` |
| `ActionOutcome::Toast` | | `ActionOutcome::Status` |
| `toast-cut` | | `status-cut` |
| `ui/shell/toast.slint` | | `ui/shell/status-line.slint`, component `StatusLine` |
| HUD | notice | `Notice`, `notice` |
| `HudWindow` (Slint and platform trait) | | `NoticeWindow` |
| `ui/shell/hud.slint`, `render/hud.rs` | | `notice.slint`, `render/notice.rs` |
| `HUD_*`, `CREATING_HUD`, `FAILURE_HUD_LIFETIME` | | `NOTICE_*`, `CREATING_NOTICE`, `FAILURE_NOTICE_LIFETIME` |
| tray notices | tray problems | `TrayStatus::problems`, `problems` in startup |
| spec capability `hud` | | `notice` |

Rejected:

- **Plain "status" for the toast.** The protocol already has `Status` for
  step and accessory states, which the status dots draw. Sharing the word
  would make "status" mean two parts.
- **Renaming the protocol's `Status` to `State`.** It is a versioned contract
  and the dots are not confusing today; the rename would only make room.
- **Keeping "tray notices".** Two different "notices" would bring back the
  confusion this change removes.

### Mechanical rename, one commit per layer

Specs and docs go in one commit, code in another. The code rename has to land
whole: Rust and Slint names meet in the generated bindings
(`set_toast_shown` comes from `Shell.toast-shown`), so a partial rename does
not compile. Identifiers are renamed with word-boundary regexes in a fixed
order (longest names first, so `ToastStyle` goes before `Toast`), files with
`git mv`. Then `cargo build`, `cargo test`, and `mise run check` decide.

Rejected: **rust-analyzer rename per symbol.** It does not reach Slint files,
comments, test names, or specs, which are most of the work.

### Spec wording edited in place

The rename changes no requirement, so the change sets `skip_specs` and edits
the main specs and the open change's deltas directly. Requirement titles
change with them, for example "A toast reports status in the ActionBar"
becomes "A status line reports status in the ActionBar". The `hud` folder
moves to `notice`, and every "see `hud`" link follows. Editing the
`launcher-v5-split-shell` deltas too keeps them matching the main specs, so
either change can be archived first.

### Contracts

No change to the extension manifest or the wire format of the view protocol.
`ActionOutcome` is a Rust enum for built-ins, not part of the protocol.

### Text

No user-visible string changes. The only strings with "HUD" are log lines
(`could not show the HUD`), which become "notice".

## Risks / Trade-offs

- [A regex hits a word it should not] → No real word in the code contains
  `toast` or `hud` as a part; review the diff for prose such as "a toast"
  becoming "a status line" (article and plural).
- [macOS code does not compile] → `platform/macos` names the HUD window.
  Check names against the vendored crates as AGENTS.md says, and let CI's
  macOS build confirm.
- [Merge pain with the open stack] → This branch stacks on `v5-closed-root`.
  Changes lower in the stack restack it with git-spice.

## Migration Plan

None. Alpha, nothing stored or configured uses the names.
