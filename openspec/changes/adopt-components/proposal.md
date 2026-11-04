# Proposal

## Why

The v2 stack added parts that existing features don't use yet: accessories on
rows, background tasks, and toast actions. Two features fit them naturally.

- **Hotkeys are invisible.** A command's or application's global hotkey only
  shows if the user opens the config file.
- **AI commands stop when their view is left.** A long agent run is lost the
  moment the user looks elsewhere.

This is change 5, the last of the v2 stack.

## What Changes

- **Root rows show their hotkey.** A command or application with a registered
  global hotkey shows it as a key chord on the right, in the platform's
  symbols.
- **AI runs become background tasks** (see `background-tasks`), once their
  prompt is ready.
  - **Leaving:** Escape or hiding the launcher no longer stops a run. Its view
    offers "Continue in background" (Enter) and "Cancel" (Ctrl+. / Cmd+.).
    Elsewhere, the busy toast reads "Running <title>…" with Cancel.
  - **Coming back:** running the same command again shows the live run.
    Another AI command replaces it once that command's prompt is ready.
  - **Finishing away from its view** (view left or launcher hidden):
    - an answer to show or paste is kept, never pasted, and a toast reads
      "Answer ready: <title>" with "Show answer" (Ctrl+Enter / Cmd+Enter);
    - an answer to copy is copied, with "Copied to clipboard";
    - a failure shows in the HUD at once and as a toast on the next open.
  - **Finishing in view:** as today, so a paste command from its hotkey still
    pastes.

## Non-goals

- No status dots for AI runs. The AI view is a markdown detail with no place
  for one, and the busy toast and loading bar already show the run.
- No "Needs permission" tag: the warning row already says so.
- No app indexing toast: the index refreshes every few minutes, and a toast
  each time would be noise.
- No accessories for root items beyond the hotkey chord.
- No change to non-AI commands: a hide still cancels them.

## Platforms

Windows and macOS.

- **Hotkey labels:** "Ctrl+Alt+V" on Windows, "⌃⌥V" on macOS, with a unit test
  each on its own platform.
- **Paste:** keeps the existing insertion path. A run that finishes away never
  pastes, so no new focus handling is needed.
- **Hand checks:** the paste-command hotkey flow on Windows. On macOS, Cmd+.
  arrives through a different native path, so it gets a Mac session.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `root-search`: a result shows its registered hotkey.
- `ai-commands`:
  - A running request is a background task.
  - A request that finishes away is kept, not pasted.
  - "A running request can be abandoned" is replaced.
- `background-tasks`: a task's "own view" replaces "its progress view", since
  an AI run's view is a detail.

## Impact

- `src/ui/input.rs` (`ResultItem.hotkey`), `src/app/mod.rs` (`result_item`
  reads the hotkey registry), `src/slint_shell/keys.rs` (`hotkey_label`),
  `src/slint_shell/render/list.rs` (the chord on root rows).
- `src/extensions/ai/extension.rs`: runs on tasks, the owned live context, the
  away endings, and the `background`, `cancel`, and `show` actions. Core hands
  it a visibility probe.
- `src/tasks.rs` (`TaskState.show`) and `src/ui/state.rs` (the "Show answer"
  toast action).
- Tests in each of those areas, and hand checks on Windows and macOS.
