# Proposal

## Why

The v2 stack added parts that existing features don't use yet: accessories on
rows, tags, background tasks, and their toasts. Two features fit them
naturally.

- **Root search rows carry no accessories.** A missing permission reads like
  any other row, and a command's hotkey is invisible unless the user opens
  the config file.
- **AI commands stop when their view is left.** A long agent run is lost the
  moment the user looks elsewhere.

This is change 5, the last of the v2 stack.

## What Changes

- **Root rows get accessories** (up to three, right-aligned, as list rows
  have).
  - A permission warning shows a "Needs permission" tag in the danger role.
  - A command with a global hotkey shows that hotkey as a key chord, drawn
    with the platform's symbols.
- **AI runs become background tasks** (see `background-tasks`).
  - **Leaving:** Escape or hiding the launcher no longer stops a run. It keeps
    going, and outside its view the busy toast reads "Running <title>…" with
    Cancel.
  - **Coming back:** running the same command again shows the live run instead
    of starting a second one. Running a different AI command still replaces it.
  - **Finishing while away:**
    - An answer to show or paste becomes a success toast, "<title> answer is
      ready", with "Show answer" (Ctrl+Enter / Cmd+Enter). It is never pasted
      into whatever app is in front by then.
    - An answer to copy is copied, with "Copied to clipboard".
    - A failure shows its sentence, in the HUD at once and as a toast on the
      next open.
  - **The run's own view:** while it is on top, no result toast is added; the
    view shows the answer.

## Non-goals

- No status dots for AI runs. The AI view is a markdown detail with no place
  for one, and the busy toast and loading bar already show the run.
- No app indexing toast: the index refreshes every few minutes, and a toast
  each time would be noise.
- No accessories declared by extensions for root items beyond these two uses.
  The plumbing allows it, but no extension asks yet.
- No change to non-AI commands: a hide still cancels them.

## Platforms

Windows and macOS. The hotkey chord is drawn per platform: modifier symbols on
macOS, "Ctrl+Alt+K" style names on Windows. The run's paste stays on the
existing insertion path. A run that finishes in the background never pastes,
so no new focus handling is needed.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `root-search`: root results can carry accessories, a permission warning shows
  a danger tag, and a command shows its hotkey.
- `ai-commands`:
  - A run outlives its view and can be cancelled.
  - Finishing away from its view reports as a toast instead of pasting.
  - Running the same command again shows the live run.

## Impact

- `src/search/mod.rs` (`Candidate.accessories`), `src/ui/input.rs`
  (`ResultItem.accessories`, `ResultItem.hotkey`), `src/app/mod.rs`
  (`result_item` fills the hotkey from the config).
- `src/extensions/permissions/mod.rs`: the tag.
- `src/slint_shell/render/list.rs`: root rows draw accessories.
  `src/slint_shell/keys.rs` gets `hotkey_label`.
- `src/extensions/ai/extension.rs`: a run on a task, the cancel and "show"
  actions, the away-from-view outcomes.
- `src/tasks.rs`: an optional "show" action on `TaskState`. `src/ui/state.rs`:
  the success toast carries it, and "the task's own view" is any view its
  owner pushed.
- Tests in each of those areas.
