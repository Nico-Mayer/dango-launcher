## Why

The UI moved from a WebView to Slint, UI state moved into Rust, and several
trims landed in a few days. An audit of all 32 specs against the code found
WebView-era wording, rules the code no longer follows, implementation detail,
duplication, and breaks of the `openspec/config.yaml` rules. The specs are the
source of truth, so they must describe the app as it is.

## What Changes

Spec text only. No code changes. Where spec and code disagree, the spec
follows the code, except for seven code bugs, which keep their spec rule and
are tracked as issues #16-#22.

- Remove WebView-era wording: "frontend", WebView2/WebKit, "every renderer",
  the reduced-motion scenario, "never the UI layer" in ai-providers.
- Align with the code:
  - The launcher opens on the display under the pointer.
  - macOS keeps a fixed launcher height; the gap is stated per platform.
  - Escape in a pushed view pops it; two-stage Escape is the root prompt only.
  - Hiding clears the view stack and drops late updates only for a
    deliberate hide or when the hold expires (launcher-shell hold).
  - Root search: a "Needs attention" section for pinned items; slow
    providers stream in and a hung one is dropped, no "time budget".
  - The hyperkey is CapsLock only; on macOS it does nothing where the tap
    cannot see it.
  - Preferences live in the config file, not the database; window state is
    not stored; Dango writes the config file only to create it.
  - Templates use the full minijinja syntax.
  - The manifest icon is optional; the unreachable "host unavailable" case
    is removed.
  - Release overrides go through `cargo release` directly; the dry run is
    specified.
  - Message texts, config key names, and small scenario facts match the code.
- Remove implementation detail and duplication: latency tooling, stream
  throttling, Theme structure rules (already in AGENTS.md), duplicate
  release-on-exit, disabled-hotkey, own-write, and permission rules.
- Fix `config.yaml` rule breaks: the 300 ms typing scenario, the 2-second
  permission poll, and requirements with platform differences but one
  scenario.
- Rewrite change-delta wording ("as before", "in this change") as current
  behaviour.

## Non-goals

- No code changes, including the bugs in #16-#22.
- No new behaviour. Missing scenarios are added only for behaviour the code
  already has.
- No restructuring of the spec set (no merging or splitting capabilities).
- No renaming of the `TAURI_SIGNING_*` CI secrets.

## Platforms

Windows and macOS. Text only, so nothing to check on either machine.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `launcher-shell`: stale wording, display choice, macOS height gap, Escape
  scope, drop latency tooling and in-process requirement, drop duplicate
  Quit rule.
- `view-protocol`: stale wording, stack and loading rules defer to the hold,
  drop reduced motion, trim announcements, detail image height, multi-line
  form submit chord.
- `design-system`: drop Theme structure rule and duplicate latency rule.
- `list-navigation`: fix the garbled "typing after clicking" scenario.
- `interface-copy`: Title Case rule, tray menu items.
- `extension-model`: optional icon, drop duplicate disabled-hotkey scenarios.
- `command-invocation`: drop change-era host wording and unreachable scenario.
- `command-hotkeys`: per-platform chord for all hotkeys, ambiguous app name.
- `root-search`: stale wording, streaming providers, typing scenario,
  pinned "Needs attention" section, drop implementation detail.
- `hyperkey`: CapsLock only, macOS secure-input behaviour.
- `keyword-expansion`: exclusion list name, Windows password detection,
  per-platform recovery, buffer size.
- `permissions`: qualitative poll timing, Accessibility list.
- `clipboard-history`: configurable limits and key names, drop duplicate
  own-write scenarios.
- `selection-and-paste`: message texts, permission request via permissions,
  drop duplication and implementation detail, excluded applications key.
- `templates`: minijinja syntax, date format.
- `quicklinks`: name the `favicons` preference.
- `ai-commands`: hold-aware hide, drop throttling detail.
- `ai-providers`: drop "never the UI layer", key file error, local server
  message, single-provider default, missing model.
- `applications`: fix contradictory scenario title.
- `window-management`: drop DWM detail, macOS scenario, delta wording.
- `configuration`: write-back limited to file creation.
- `autostart`: broken config at startup leaves the login entry.
- `release-pipeline`: stale title, override path, dry run.
- `local-checks`: stale title, delta wording.
- `local-store`: drop preferences requirement, startup wording, split
  location scenario per platform.
- `records-storage`: drop window state.

## Impact

`openspec/specs/` only. No code, config, manifest, or view protocol change.
