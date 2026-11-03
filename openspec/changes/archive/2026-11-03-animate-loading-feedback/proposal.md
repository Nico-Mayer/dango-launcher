## Why

Loading feedback is fully static: the accent bar under the SearchBar, the busy
toast's dot, and the AI view's "Working…" text never change. While an AI
command waits for its provider, often for many seconds with an agent program,
a slow provider and a stuck launcher look the same. The user cannot tell that
work is going on, or what it is waiting for.

## What Changes

- The SearchBar's loading bar shows motion: a short accent segment sweeps along
  the bar's bottom edge while any list, grid, or detail view is marked loading.
  It is the only motion in the interface; everything else still changes in one
  frame.
- An AI command's view reads "Waiting for {provider}…" until the first text
  arrives, in place of "Working…". `{provider}` is the name the user gave the
  provider in the config file. Once text arrives, the growing answer and the
  sweep show that it is being written.

## Non-goals

- No other motion: no entrance effects, no animated selection, hover, toast,
  dot, or action panel.
- No spinner icon.
- No elapsed time, progress percentage, or estimate.
- No change to the view protocol, and no command-defined busy toast text. The
  busy toast keeps saying "Loading…".
- No setting to turn the motion off, and no reading of the OS reduce-motion
  setting.
- No change to when a view counts as loading.

## Platforms

Windows and macOS. The sweep is drawn by Slint in the launcher window and the
AI text comes from shared Rust code, so the behaviour is the same on both. No
platform code changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `design-system`: the loading bar moves while a view is loading; "Nothing
  animates" gains that one exception.
- `ai-commands`: before the answer arrives, the view names the provider it is
  waiting for.

## Impact

- UI: `ui/shell/search-bar.slint` (the sweep), `ui/theme.slint` (its size and
  pace).
- AI: `src/extensions/ai/extension.rs` replaces the "Working…" body.
- Tests: `src/slint_shell/tests.rs`, `src/extensions/ai/extension.rs`.
- No new dependency, no protocol change.
