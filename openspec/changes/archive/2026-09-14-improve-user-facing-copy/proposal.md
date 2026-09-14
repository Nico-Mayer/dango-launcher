## Why

Dango's user-facing text grew one string at a time, per feature, with no shared
standard. The result is visible every day: the failure banner shows Rust error
strings verbatim ("that command is no longer available", "the window refused to
move", "Finder answered '3'"), lowercase and without a next step; the tray says
"see log" without saying where the log is; a form's validation reads "give it a
name"; the snippets error view is titled "That did not work"; and casing mixes
Title Case ("Paste to Active App", "Copy Path") with sentence case ("Nothing
copied yet") on the same screen. None of it is broken, but none of it was
written for the person reading it, and each new feature so far has invented
its own tone.

Two things fix this for good. First, a one-time pass over every string the
user can see, written with the `ux-writing` skill's four standards: purposeful,
concise, conversational, clear. Second, a spec that pins the standard down, so
every future change that adds or edits interface text is held to the same
rules and the skill is used to write it.

Milestone: M7 - polish. It is the first M7 change and needs nothing from M6.

## What Changes

- A new `interface-copy` capability spec defines the observable rules for all
  text Dango shows: casing, action labels, error message shape, empty states,
  confirmations, tray notices, and consistent terminology. Later changes
  inherit it the way they inherit `view-protocol`.
- `openspec/config.yaml` gains a convention: any change that adds or edits
  user-facing text writes it with the `ux-writing` skill and against the
  `interface-copy` spec, and its design names the strings it introduces.
- Every existing user-facing string is rewritten where it falls short. Groups:
  - **Failure messages** that reach the banner become complete sentences that
    say what failed and, where there is one, what to do. Error types whose
    messages are shown to the user (`InvocationError`, `TextError`,
    `WindowError`, `SystemError`, `LaunchError`, snippet `StoreError`, clipboard
    `HistoryError`, the applications extension's action failures) get
    user-facing wording; internal detail such as an `AXError` code or an OS
    error moves to the log.
  - **Tray notices** name the log file, `dango.log` in the config directory,
    instead of "see log", and the launcher hotkey notice says what to change.
  - **Empty states and status lines** ("That did not work", "Asks for nothing",
    "Will ask for:", "Nothing here", "Working...", "Loading...") are reworded
    to name the thing and the next step.
  - **Action labels** move to sentence case, verb first, and use "Delete" where
    the action is permanent: snippet and quicklink "Remove" become "Delete",
    clipboard "Remove from History" becomes "Delete from history", the
    applications "Launch" becomes "Open", "Reveal in Finder" and "Reveal in
    File Explorer" become "Show in Finder" and "Show in File Explorer", and
    both clipboard "Paste to Active App" and snippet "Insert" become "Paste".
  - **Form labels and placeholders** are tidied: "Keyword (optional)" keeps its
    optional marker, the body field gets a label that says what it holds, the
    root search placeholder loses its ellipsis.
  - **Tray menu** items read "Show or hide Dango" and "Quit Dango", with the
    config status as "Config file loaded" or "Config file has an error".
- Command titles in root search stay Title Case. They sit in one list with
  application names such as "Visual Studio Code" and are searched by name, so
  they read as names, not as sentences. This is the one deliberate departure
  from the skill's default, recorded in the spec.
- **BREAKING** for muscle memory only: a user who reads the footer as "Launch"
  or "Paste to Active App" will see "Open" and "Paste". Action identifiers,
  config keys, and the view protocol are unchanged.

## Capabilities

### New Capabilities

- `interface-copy`: the writing standard for everything Dango puts on screen -
  root search, pushed views, the failure banner, forms, empty states,
  confirmations, and the tray menu. Casing, action label shape, error message
  shape, terminology, and the rule that a failure is never a raw internal
  error string.

### Modified Capabilities

<!-- None. Existing specs describe behaviour in prose ("the failure is shown",
     "explains that the permission is needed") and pin no exact strings, so no
     requirement changes. The new capability adds rules on top. -->

## Impact

- `src-tauri/src/invocation.rs`, `text/mod.rs`, `platform/mod.rs`,
  `platform/{macos,windows}/{window,system,text,apps}.rs`,
  `extensions/{applications,clipboard,snippets,system,window_management}/`,
  `extension/mod.rs`: error message text and action titles. No signatures
  change; only string literals and, in a few places, which detail goes to the
  message and which goes to `eprintln!`.
- `src-tauri/src/lib.rs`: tray notices, tray menu items, config status line.
- `src/App.svelte`, `src/lib/ProtocolView.svelte`, `src/lib/FormFields.svelte`:
  placeholders, empty states, status lines, footer fallback label.
- `openspec/config.yaml`: one new convention under Conventions and one new
  design rule.
- Tests that assert on message text (`extensions/system/mod.rs`,
  `extensions/snippets/extension.rs`, `invocation.rs`, `templates/mod.rs`)
  are updated to the new strings.
- No new dependency. No change to `manifestVersion` or `protocolVersion`. No
  change to `config.json` keys or `docs/config.example.jsonc`.

Platforms: macOS and Windows, both fully. Most strings are shared. The ones
that name a platform component (Finder and File Explorer, Trash and Recycle
Bin, Option and Alt, Cmd and Ctrl, the Accessibility permission) already have
per-platform variants and keep them. The only per-platform verification is
that each platform's own error paths show the new wording.

## Non-goals

- No new UI surface. No toast, no notification, no settings screen. The
  failure banner, the tray, and the empty state stay the only places a message
  can appear.
- No `placeholder` field on the list view. Pushed lists keep the generic
  "Search" placeholder; giving each command its own would touch the view
  protocol and is a separate change.
- No inline form validation. Snippet and quicklink rules are still checked on
  save and reported in the banner; the change is to the words, not the timing.
- No localisation. English only, as before.
- No change to the log format or location, only to how the UI refers to it.
- No renaming of command titles, extension names, or config keys.
