## Why

A snippet today costs a hotkey, a search, and a confirmation. That is fine for
something used weekly and too slow for the thing the author actually wants,
which is typing `;email` in any application and having their address appear.
The launcher is the wrong surface for a snippet used twenty times a day: the
fastest way to reach text you type constantly is to type it.

This belongs to **M5 - keys** from `openspec/ROADMAP.md`, because its mechanism
is the global key monitor M5 owns. It is proposed now, ahead of the rest of M5,
for two reasons. The expensive half already exists: `add-text-plumbing` built
and verified the path that puts text into another application, so this change
adds a way to trigger it rather than a way to perform it. And the findings from
that change are fresh, including the ones that cost several attempts to get
right, which is exactly when the next thing built on them is cheapest.

The rest of M5, the hotkey recorder and conflict detection, stays where it is.

## What Changes

- A snippet may declare a **keyword**. Typing it in any application replaces it
  in place with the snippet's text: the keyword is deleted and the text is
  inserted through the path M3 already built and verified.
- A **key monitor** service, watching typed characters system-wide. This is the
  most invasive component in the project, and most of this change is about what
  it is not allowed to see or keep.
- A small rolling buffer of recent characters, in memory only, cleared
  aggressively: on a focus change, on any key that is not a character, after a
  pause, and whenever the platform says a password is being typed.
- Expansion is **skipped entirely while secure input is active**, and in
  applications the user has excluded, reusing the exclusion list the clipboard
  history already has.
- Covered on Windows and macOS. macOS is built and verified first; the Windows
  half lands as code plus confirmation tasks, the arrangement the last two
  changes used.

## Capabilities

### New Capabilities

- `keyword-expansion`: what a keyword is and when it matches, what happens to
  the typed keyword, what the monitor is permitted to observe and keep, when
  expansion is deliberately skipped, and how the whole thing is turned off.

### Modified Capabilities

- `snippets`: a snippet gains an optional keyword, which the create and edit
  forms accept and which must be unique. Confirming a snippet from root search
  is unchanged.

## Impact

- **Affected code**: `src-tauri/src/platform/` (an event tap on macOS, a
  low-level keyboard hook on Windows, behind one trait),
  `src-tauri/src/extensions/snippets/` (the keyword, its matching, and the
  service), `src-tauri/src/text/` (deleting the keyword before inserting).
- **Database**: a forward-only migration adding a nullable `keyword` column to
  `snippets`, with a unique index over the live rows.
- **Dependencies**: none. The evaluation is in design.md and its conclusion is
  unusual: the maintained crates report which *key* was pressed, and matching
  typed text needs which *character* was produced, which is the only hard part
  of the job. Both platforms can already answer that through crates this project
  depends on today.
- **Permissions**: no new ones. macOS needs Accessibility, which
  `add-text-plumbing` already made a visible, explained state.
- **Latency**: the monitor sits on the system's input path. A slow callback
  delays every keystroke in every application, so the budget here is not about
  Dango feeling fast, it is about the machine not feeling broken.

## Non-goals

- **No hotkey recorder, no conflict detection, no hyperkey.** The rest of M5 is
  untouched. This takes the key monitor and nothing else that milestone owns.
- **No expansion for quicklinks.** A URL is not something you type mid-sentence,
  and a quicklink that needs a query cannot be answered without a prompt, which
  defeats the point of expanding in place.
- **No placeholders in an expanded snippet.** A snippet whose template needs an
  argument cannot be expanded in place, because asking would mean showing a
  window, which is the thing being avoided. Such a snippet keeps working from
  root search and is refused a keyword.
- **No rich matching.** One exact keyword per snippet. No regular expressions,
  no case-insensitive matching, no fuzzy matching, no multiple keywords.
- **No undo.** Expansion is a paste like any other, so the application's own
  undo is what reverses it. Dango does not track what it expanded.
- **No key remapping or interception.** The monitor observes and never swallows
  or rewrites a keystroke, except for the backspaces it sends deliberately.
- **No Linux.** Out of scope for the project.
