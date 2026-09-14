## Context

See proposal.md - Why for the motivation. What shapes the approach:

- Text reaches the user by four routes, and none of them rewrites what it is
  given:
  1. **The failure banner.** `InvocationContext::fail(message)` and
     `ActionOutcome::Failed(message)` in Rust become `EVENT_FAILED` or a
     `{ kind: "failed", message }` response, and `src/App.svelte` renders the
     string as is under a `circle-alert` icon. Almost every call site passes
     `error.to_string()`, so the banner shows the `thiserror` `Display` text of
     `InvocationError`, `TextError`, `WindowError`, `SystemError`,
     `LaunchError`, `IndexError`, `RecordError`, `HistoryError`, and
     `TemplateError`. Those strings follow the Rust API convention for error
     messages: lower case, no trailing period, written to be wrapped by a
     caller. Nothing wraps them.
  2. **View trees.** `ListItem.title`, `Action.title`, `FormField.label`,
     `EmptyState`, and `DetailView.markdown` are authored in each extension and
     rendered by `src/lib/ProtocolView.svelte` and `src/lib/FormFields.svelte`.
     The snippets extension also turns a store error into an empty state
     through `failure_tree`, titled "That did not work".
  3. **Frontend literals.** Placeholders, "No results", "Nothing here",
     "Working...", "Loading...", "Asks for nothing", "Will ask for:", the footer
     fallback "Select", and the protocol-version error live in the three
     Svelte files.
  4. **The tray menu.** `lib.rs` collects startup notices into disabled
     `MenuItem`s, plus a live config status item and two actionable items.
     The log they refer to is `dango.log` in the config directory, written by
     `log_config`.
- Some error variants embed operating-system detail in the sentence:
  `SystemError::Failed(format!("the system refused to lock ({error})"))`,
  `WindowError::Failed(format!("{attribute} was not an AXValue"))`,
  `"Finder answered '{}'"`. The detail is useful in a log and noise in a
  banner.
- `TemplateError::Parse` carries minijinja's own message. It says where the
  problem is, which the user needs while editing, but reads as a compiler
  diagnostic.
- Command titles come from each extension's manifest and are Title Case
  throughout ("Empty Trash", "Clipboard History", "Left Half"). They are listed
  alongside indexed application names, which are Title Case by nature.
- The frontend already knows the query in both list surfaces and knows the
  clicked item's title in `confirm(item)` when it sets `working`.
- Tests pin a handful of strings: `store.rs` asserts on "give it a URL" and
  "give it a valid URL", `invocation.rs` on "no good", and several tests match
  on `Outcome::Failure(_)` without reading the text.
- No i18n layer exists and none is wanted. English literals in place are the
  right cost for a one-person launcher.

## Goals / Non-Goals

**Goals:**

- Every string the user can see meets the `interface-copy` spec, with one
  authoritative before-and-after table in this document so the apply phase is
  a transcription, not a writing exercise.
- User-facing wording lives where the string already lives. No new layer, no
  message catalogue, no mapping function.
- Debugging detail is kept, but moved to stderr or `dango.log`.
- The standard is enforced going forward by `openspec/config.yaml`, not by
  memory.

**Non-Goals:**

- Localisation or a string table.
- Changing what a failure does, only what it says. Banners stay banners, empty
  states stay empty states, the tray stays the only startup surface.
- Any change to action ids, config keys, `manifestVersion`, or
  `protocolVersion`.

## Decisions

### User-facing wording is the error type's `Display`

The `Display` string of every error type that reaches the banner becomes the
user-facing sentence: capitalised, a full stop, a next step where there is one.
Variants that today interpolate an OS error keep that detail in a field but
leave it out of `Display`; the site that constructs the variant logs the detail
with `eprintln!("[dango] ...")` first. Error types that only ever reach the log
(`ConfigError`, `HotkeyError`, `StoreError`, `MigrationError`,
`ManifestError`) keep the Rust convention and are untouched.

Alternatives rejected:

- **A `user_message()` method or `UserFacing` trait beside `Display`.** Every
  enum would carry two strings per variant for one consumer. The log never
  reads these `Display` strings today; call sites that want detail already
  `eprintln!` it. Two strings is a maintenance cost with no second reader.
- **Capitalise and punctuate at the frontend.** Hides the problem instead of
  fixing it, breaks on messages that start with a name ("dango.log"), and
  leaves the "what to do" half unwritten.
- **Wrap at the `ctx.fail` call site.** Spreads the wording over the callers
  and leaves `error.to_string()` doing something different at each one.

### Sentence case everywhere, Title Case for command and extension titles

Everything the user reads is sentence case, following the skill's default,
except the titles of commands and extensions, which stay Title Case.

Why the exception: root search is one list holding "Visual Studio Code",
"Empty Trash", and "Clipboard History". Application names are Title Case by
nature. A command title in sentence case next to them reads as an instruction
rather than a name, and the user searches for these by name. Raycast, the
stated model, does the same.

Alternatives rejected:

- **Sentence case for commands too.** "Empty trash" and "Lock screen" between
  "Visual Studio Code" and "Windows Terminal" look like two kinds of row in one
  list.
- **Title Case for actions and labels too.** This is what the code does today
  and is the skill's named anti-pattern: "Paste to Active App" and "Copy Path"
  read as headings, and a Title Case sentence in the banner is not possible,
  so the two styles would meet on one screen anyway.

### Verbs are chosen by effect, and one thing has one name

"Delete" when the thing is gone for good (snippets, quicklinks, history
entries), "Open" for launching an app or URL, "Paste" for putting text into the
previous app, "Copy" for putting it on the clipboard, "Show in" for the file
manager (Finder's own menu says "Show in Finder"; File Explorer's own phrasing
is "Open file location", but "Show in File Explorer" keeps one verb across the
two platforms). Snippets and history entries share "Paste" because they do the
same thing to the same target. The terminology list in the spec is the
reference; the table below applies it.

### The frontend fills in what it already knows

Root and pushed lists show `No results for “<query>”`, using the query that is
already in component state, so the empty state confirms what was searched. The
working line becomes `Running <command title>…`, from the `ResultItem` that
`confirm` already holds; a new `workingTitle` state replaces the boolean.
Long queries are clipped by the existing `truncate` styling, not by code.

Alternative rejected: keeping the generic "No results" and "Working...". Both
are correct and both leave the user to infer the subject. Naming it costs one
interpolation.

### Progress uses the ellipsis character, prompts do not

"Loading…" and "Running <title>…" end in a single `…` (U+2026), because an
ellipsis on a status line signals something in flight. Placeholders are
prompts, not progress, so "Search apps and commands" and "Search" drop the
three dots the code has today.

### Template errors keep the engine's location, behind a plain lead-in

The form's inspection line and any save failure caused by a template read
`This template can't be read: <minijinja message>`. The engine's text is the
only thing that says which brace is unclosed, and the field is being edited as
it is shown, so the location is worth the jargon.

Alternative rejected: replacing the engine text with a fixed sentence. The user
then has to find the problem in a five-line template by eye.

### Tray notices are short, name the log, and say what to change

`MenuItem` labels do not wrap, so notices stay under about 50 characters. Every
notice whose detail went to the log ends in `see dango.log`, the file's actual
name, in place of "see log". The launcher hotkey notice names the chord and the
config key to change, `launcher.hotkey`.

Alternative rejected: making the notice a clickable item that opens the log.
It is new behaviour with its own spec, and the notice is enough to find a file
by name in a directory the user already edits.

### `openspec/config.yaml` carries the convention and a design rule

Two additions, so that future planning sessions hit the rule before writing a
string:

Under `Conventions`:

```
- Anything the user reads is written with the `ux-writing` skill and against
  the `interface-copy` spec: sentence case except command and extension
  titles, verb-first action labels, failures that say what failed and what to
  do, no internal identifiers or error codes on screen. Every new or changed
  user-facing string is quoted in the change's design.
```

Under `rules.design`:

```
- A change that adds or edits text the user can see quotes every new or
  changed string, written with the ux-writing skill and checked against
  openspec/specs/interface-copy/spec.md.
```

Alternative rejected: leaving the rule to the spec alone. Specs are read when
a change touches their capability; a casing rule has to be seen by every
change, and `context` and `rules` are what every session reads.

### A `Display` test per user-facing error type

Each error enum whose `Display` reaches the banner gets one unit test that
constructs every variant and asserts the text starts with an uppercase letter
and ends with a full stop. It is cheap, it fails the moment a new variant is
added with the Rust convention out of habit, and it is the only mechanical
check available: neither clippy nor a lint knows which strings are
user-facing.

Alternative rejected: a repo-wide `rg` over string literals as a CI step. Too
many false positives in SQL, test fixtures, and log lines to be a gate.

### No new surface, no primitive change

Every string lives in an existing surface. The root and pushed lists remain
Bits UI `Command.Root`, with the placeholder on `Command.Input` and the empty
state as a plain row inside `Command.Viewport`, as before. The banner, footer,
and working line are static status regions, not interactive, and stay plain
elements. Nothing in the view protocol changes shape; the frontend keeps
rendering `EmptyState.title` and `description` exactly as sent.

## The strings

Everything in the left column is the current literal. Everything in the right
column is what ships. A blank right column means unchanged.

### Frontend

| Where | Now | After |
| --- | --- | --- |
| Root placeholder | `Search for apps and commands...` | `Search apps and commands` |
| Pushed list placeholder | `Search...` | `Search` |
| Root empty | `No results` | `No results for “{query}”` |
| Pushed list, filtered empty | `No results` | `No results for “{query}”` |
| Pushed list, no declared empty state | `Nothing here` | `Nothing to show` |
| Pushed list loading | `Loading...` | `Loading…` |
| Root working line | `Working...` | `Running {command title}…` |
| Footer fallback | `Select` | |
| Footer panel label | `Actions` | |
| Protocol error | `This view needs a newer version of Dango.` / `Press Escape to go back.` | |
| Template inspection, arguments | `Will ask for: {a, b}` | |
| Template inspection, none | `Asks for nothing` | `Nothing to fill in` |
| Template inspection, error | `{engine message}` | `This template can't be read: {engine message}` |

### Actions and labels

| Where | Now | After |
| --- | --- | --- |
| Applications | `Launch` | `Open` |
| Applications, macOS | `Reveal in Finder` | `Show in Finder` |
| Applications, Windows | `Reveal in File Explorer` | `Show in File Explorer` |
| Applications | `Copy Path` | `Copy path` |
| Clipboard | `Paste to Active App` | `Paste` |
| Clipboard | `Copy to Clipboard` | `Copy` |
| Clipboard | `Remove from History` | `Delete from history` |
| Snippets | `Insert` | `Paste` |
| Snippets, quicklinks | `Open` / `Copy` / `Edit` | |
| Snippets, quicklinks | `Remove` | `Delete` |
| Snippet form body label | `Snippet` | `Text` |
| Quicklink form body label | `URL` | |
| Form labels | `Name` / `Keyword (optional)` | |
| Form primary action | `Save Snippet` / `Save Quicklink` | `Save snippet` / `Save quicklink` |
| System, quit list | `Quit` | |
| System, trash confirm | `Cancel` / `Delete {n} {noun}` | |
| System, trash body | `Permanently delete {n} {noun}?` / `This cannot be undone.` | |

Command titles (`Lock Screen`, `Empty Trash`, `Empty Recycle Bin`, `Quit
Application`, `Clipboard History`, `Create Snippet`, `Search Snippets`,
`Create Quicklink`, `Search Quicklinks`, the window-management set) and
extension names (`Applications`, `Clipboard`, `System`, `Snippets`,
`Quicklinks`, `Window Management`) are unchanged.

### Empty states

| Where | Now | After |
| --- | --- | --- |
| Clipboard, title | `Nothing copied yet` | |
| Clipboard, description | `What you copy from now on will show up here, unless it came from an excluded application.` | `Text and images you copy will show up here. Copies from excluded apps are left out.` |
| Snippets | `No snippets yet` / `Create one to keep text you type again and again.` | |
| Quicklinks | `No quicklinks yet` / `Create one to reach a URL you visit often by name.` | |
| Snippets, records file unreadable | `That did not work` / `{error}` | `Couldn't read your snippets` or `Couldn't read your quicklinks` / `{error}` |
| System, quit list | `Nothing is running` | `No apps to quit` |

### Failures

| Type and variant | Now | After |
| --- | --- | --- |
| `InvocationError::Unavailable` | `that command is no longer available` | `That command isn't available any more. Search again to refresh the list.` |
| `InvocationError::Unsupported` | `that command cannot run in this build` | `That command isn't available on this platform.` |
| Invoker, command panicked | `the command crashed` | `That command stopped unexpectedly. Try again.` (the panic goes to stderr, not `dango.log`, so the log is not named) |
| Host, no such extension | `that extension is not available` | `That extension is turned off.` |
| Host, no actions / unknown action (all extensions) | `this extension has no actions`, `unknown action '{x}'`, `that is not something a snippet can do` | `That action isn't available.` |
| `TextError::PermissionMissing` | `Dango needs the Accessibility permission to do that` | `Dango needs the Accessibility permission to paste. Grant it in System Settings, Privacy & Security.` |
| `TextError::NoTarget` | `there is no application to put that into` | `Nothing to paste into. Switch to an app first.` |
| `TextError::Clipboard` | `the clipboard could not be used` | `Couldn't use the clipboard. Try again.` |
| `TextError::TargetUnavailable`, elevated (Windows) | `that window belongs to an elevated program, which Dango cannot reach` | `Dango can't paste into programs running as administrator.` |
| `TextError::TargetUnavailable`, foreground (Windows) | `the previous window would not come back to the foreground` | `The previous app didn't come back to the front. Click into it and try again.` |
| `TextError::TargetUnavailable`, keystroke (both) | `{enigo error}`, `the main thread did not run the keystroke` | `Couldn't send the keystroke. Try again.` with the detail logged |
| `WindowError::NoTarget` | `there is no window to move` | `No window to move. Focus one first.` |
| `WindowError::Unreachable`, elevated (Windows) | `that window belongs to an elevated program, which Dango cannot reach` | `Dango can't move windows of programs running as administrator.` |
| `WindowError::Unreachable`, AX not implemented or attribute missing (macOS) | `that application does not let Dango move its windows`, `that window has no {attr}` | `That app doesn't let Dango move its windows.` |
| `WindowError::Failed`, permission (macOS) | `Dango needs the Accessibility permission to move windows` | `Dango needs the Accessibility permission to move windows. Grant it in System Settings, Privacy & Security.` |
| `WindowError::Failed`, read frame or display (both) | `could not read the window's frame`, `could not read the display`, `{attr} was not an AXValue`, `{attr} could not be read`, `{attr} could not be encoded`, `the focused window could not be read` | `Couldn't read the window's position. Try again.` with the detail logged |
| `WindowError::Failed`, move refused (Windows) | `the window refused to move` | `That window didn't move. It may have a fixed size.` |
| `WindowError::Failed`, other AX error (macOS) | `that window could not be reached ({code})` | `Couldn't reach that window. Try again.` with the code logged |
| `SystemError::Failed`, lock (both) | `the system refused to lock ({error})` | `The screen didn't lock. Try again.` with the error logged |
| `SystemError::Failed`, no lock symbol (macOS) | `this version of macOS has no lock entry point` | `Lock screen isn't supported on this version of macOS.` |
| `SystemError::Failed`, sleep (both) | `the system refused to sleep ({error})`, `{program} refused the request` | `Sleep didn't start. Try again.` with the error logged |
| `SystemError::Failed`, trash count (both) | `the recycle bin could not be read ({error})`, `Finder answered '{x}'` | `Couldn't check the Recycle Bin.` / `Couldn't check the Trash.` with the detail logged |
| `SystemError::Failed`, empty trash (both) | `the recycle bin could not be emptied ({error})`, Finder's own error text | `Couldn't empty the Recycle Bin.` / `Couldn't empty the Trash.` with the error logged |
| `SystemError::Failed`, not an app (both) | `that is not an application` | `That isn't an app Dango can quit.` |
| `SystemError::Failed`, no window (Windows) | `that application has no window left to close` | `That app has no window left to close.` |
| `SystemError::NotRunning` | `that application is no longer running` | `That app is no longer running.` |
| `SystemError::Unsupported` | `{0} is not implemented on this platform yet` | `{0} isn't available on this platform.` |
| Applications, gone | `application is no longer in the index`, `application no longer exists` | `That app is no longer installed. It's been removed from results.` |
| Applications, no path | `this application has no file path` | `This app has no file path to copy.` |
| Applications, reveal failed | `{io error}` | `Couldn't show {name} in Finder.` / `Couldn't show {name} in File Explorer.` with the error logged |
| `LaunchError::Failed` (Windows shell) | `shell refused to launch {name}` | `Couldn't open {name}. Try again.` |
| `RecordError::NoName` | `give it a name` | `Enter a name.` |
| `RecordError::NoBody` | `give it a URL`, `give it some text` | `Enter a URL.` / `Enter the snippet text.` |
| `RecordError::InvalidUrl` | `give it a valid URL` | `Enter a valid URL, such as https://example.com.` |
| `RecordError::Gone` | `that one is no longer there` | `That one no longer exists. Go back to refresh the list.` |
| `RecordError::Parse` | `the records file is not valid JSON: {0}` | `The file isn't valid JSON: {0}` |
| `RecordError::KeywordNeedsPlainSnippet` | `a snippet with a fill-in-the-blank cannot have a keyword` | `A snippet that asks for input can't have a keyword.` |
| `RecordError::KeywordTaken` | `another snippet already uses the keyword '{0}'` | `Another snippet already uses the keyword {0}.` |
| `RecordError::Template`, `TemplateError::Parse` | `{engine}` | `This template can't be read: {engine}` |
| `TemplateError::Render` | `{engine}` | `This template couldn't be filled in: {engine}` |
| Snippets, no opener | `nothing here can open a URL` | `Dango can't open URLs on this platform.` |
| `HistoryError::TooLarge` | `that item is too large to keep` | `That entry is too large to keep.` |
| `HistoryError::Gone` | `that entry is no longer there` | `That entry was already deleted.` |
| `HistoryError::Failed` | `{sqlite or io error}` | `Couldn't read the clipboard history. Try again.` with the error logged |

### Tray

| Where | Now | After |
| --- | --- | --- |
| Store recovered | `Database was corrupt, started empty` | `Database was reset, see dango.log` |
| Store unavailable | `Database unavailable, see log` | `Database unavailable, see dango.log` |
| Records file | `{file} has an error, see log` | `{file} has an error, see dango.log` |
| Launcher hotkey, macOS | `Option+Space unavailable, already in use` | `Option+Space is in use, set launcher.hotkey` |
| Launcher hotkey, Windows | `Alt+Space unavailable, already in use` | `Alt+Space is in use, set launcher.hotkey` |
| Command hotkeys | `Hotkey conflict, see log` | `Some hotkeys are already in use, see dango.log` |
| Hyperkey | `Hyperkey unavailable, see log` | `Hyperkey didn't start, see dango.log` |
| Config status | `Config error, see log` / `Config loaded` | `Config file has an error, see dango.log` / `Config file loaded` |
| Menu | `Toggle Dango` / `Quit Dango` | `Show or hide Dango` / `Quit Dango` |

## Risks / Trade-offs

- **Windows strings land unverified.** Windows cannot be compiled locally, and
  the Windows platform files carry a third of the failure strings. → Windows
  edits are literal-only with no signature change, the `Display` tests cover
  the shared enums on both CI runners, and the Windows verification tasks
  exercise each Windows-only path by hand.
- **Existing muscle memory for "Launch" and "Insert".** → The footer shows the
  new label on the very first use, and the ids behind the actions are
  unchanged, so a hotkey or config that refers to an action keeps working.
- **Tests that match exact text.** Two `store.rs` tests and one `invocation.rs`
  test assert on strings that change. → They are updated in the same task as
  the string, and the `Display` tests replace them as the guard.
- **The Title Case exception drifts.** A future change may write a command
  title in sentence case or an action in Title Case. → The spec has a scenario
  for each, and the config rule requires the design to quote new strings.
- **Losing detail from the banner.** Moving OS errors to the log means a user
  reading the banner sees less. → The banner never helped with that detail
  anyway, and the log is now named on screen wherever it holds more.
- **Quoted queries can be wide.** `No results for “…”` with a long query. →
  The row already truncates; the query is still visible in the input above.
