# Windows native verification: tasks 6.2, 6.4, 6.6

Environment: Windows 11 Pro 10.0.26200, AMD Ryzen 7 9800X3D, 2560x1440 at 100
percent scaling (96 DPI), WebView2 153.0.4234.32, Node v24.21.0, rustc 1.98.0,
2026-09-17. Release binary from `npx tauri build --no-bundle` (2m 29s), run with
`DANGO_MEASURE=1` and stderr redirected to `activation.log`. The user's own
config was used unchanged: launcher hotkey `mod+space` (Ctrl+Space), four
quicklinks, three snippets, AI provider `claude` (CLI).

Input was synthesized with SendInput (Python `uiautomation` 2.0.29 and
.NET SendKeys); accessibility state was read through UI Automation from the
WebView2 tree. Screenshots are 720x400 window captures with a 24px margin so
the transparent corners show the desktop behind them.

## Build and activation timing (6.6)

25 hotkey activations in a row, Escape between each, root view with warm
suggestions:

| min | median | p90 | max | over 80ms |
| --- | ------ | --- | --- | --------- |
| 17.6ms | 21.7ms | 23.1ms | 23.8ms | 0 |

All other activations in `activation.log` (pushed lists, forms, carried
failure) were between 16.1ms and 28.9ms. The window is painted at its final
position with no entrance effect.

Hidden window: after hiding from the root view with a carried failure, five
seconds of sampling showed 0.0ms CPU across `dango.exe` and all six WebView2
processes. No loop survives the hide.

Streaming trace: not obtained. See "Open" below.

## Interactive checks (6.2)

- **Root view** (`win-root.png`): 64px search header, 56px rows, group heading,
  blue selection on the first suggestion, footer with `Open ↵` and the disabled
  `Actions Ctrl K` trigger (root commands have no secondary actions). Corners are
  transparent; the desktop behind shows through with no white fringe.
- **Ctrl+K on a root command without actions** does nothing and the trigger
  reports `disabled=true`; Escape then dismisses the launcher. This is the
  specified behaviour, recorded because the first attempt looked like a defect.
- **Application result** (`win-app-brave.png`): typing `brave` shows the Brave
  entry with its extracted icon untinted and contained, and the executable path
  as subtitle.
- **Action panel from a result** (`win-app-panel-open.png`, `-down.png`,
  `-closed.png`, `win-app-typed-after-panel.png`): Ctrl+K opens the `Actions`
  listbox anchored above the footer trigger with `Open` selected and focused
  through `aria-activedescendant`. ArrowDown moves to `Show in File Explorer
  Ctrl+R`; the parent row keeps `selected=true`. Escape closes only the panel:
  the launcher stays visible, the `Actions` listbox is gone from the tree, focus
  returns to the search combobox, and the next keystroke lands in the query
  (`bravex`). Escape clears the query, Escape hides.
- **Pushed list with favicons** (`win-quicklinks-list.png`): Search Quicklinks
  shows GitHub, ChatGPT and Google favicons untinted in their frames and the
  generic link glyph for the host whose favicon could not be fetched. Focus is
  on the first row.
- **Panel over a pushed list** (`win-quicklinks-panel.png`, `-last.png`,
  `-closed.png`): four actions (`Open`, `Copy`, `Edit`, `Delete`). Six
  ArrowDown presses stop at `Delete` without wrapping and without moving the
  parent selection (GitHub stays selected). Escape closes the panel and focus
  returns to the selected row; Escape again pops to root
  (`win-quicklinks-back-root.png`); two more hide.
- **Form** (`win-form.png`, `win-form-error.png`, `win-form-panel.png`,
  `win-form-back.png`): Create Snippet opens with focus in `Name`. Fields are
  exposed as `Name`, `Text` (multiline) and `Keyword (optional)`. Typing
  `Hello {{ unclosed` into `Text` marks that field `invalid=true` and shows the
  red error line with its icon under the field. Ctrl+K in the form opens the
  panel with `Save snippet`; Escape closes it and focus returns to the `Text`
  field with its invalid state intact; Escape leaves the form.
- **Carried failure** (`win-failure.png`): running Explain This with nothing
  selected hides the launcher to read the selection; the next activation shows
  the root view with `Select some text first, then run this command.` in the
  footer failure line. The status region is `role=status`, `aria-live=polite`,
  `aria-atomic=true`.
- **Long action list**: no built-in item has more than four actions on this
  machine, so vertical overflow of the panel could only be checked in the
  browser fixture (`../actions/`).

## Accessibility state read through UI Automation (6.4, partial)

Not NVDA. These are the names and states WebView2 exposes, read with UI
Automation, which is what NVDA would consume:

- Search field: `ComboBox` named `Search apps and commands`, `haspopup=listbox`,
  `expanded=true`.
- Result rows: `ListItem` named from title and subtitle, for example `Left Half
  Window Management`; the selected row carries `selected=true` and is reported
  as the focused element while the prompt holds DOM focus.
- Action panel: `listbox` named `Actions`; options are named from the label and
  shortcut, for example `Show in File Explorer Ctrl+R`; the selected option is
  reported as focused.
- Footer trigger: `Button` named `Actions`, `haspopup=dialog`, `disabled=true`
  when the selection has no actions.
- Form fields: `Edit` controls named by their labels, `multiline=true` on the
  textarea, `invalid=true` on the field with a template error.
- Failure and status: one `role=status` region with `live=polite` and
  `atomic=true`; no `aria-busy` node exists while idle.

## Open

- **Streaming trace on the release build.** Every synthetic run of an AI
  command against a throwaway selection window ended with the launcher hidden
  and nothing shown. A trace build showed why: the harness itself changed the
  foreground window during the run (terminal, browser, Alt+Tab), so the
  selection handoff yielded to the wrong window. With a real selection and
  physical input the author confirmed the loading and streaming flow in a dev
  build. A performance trace of that flow on the release build is still to be
  taken by hand.
- **200 percent display scaling** was checked by the author on this machine.
- **NVDA and reduced motion** were checked by the author on this machine.

## Observations outside this change

- The first launch printed `could not register the launcher hotkey: HotKey
  already registered` because a previous Dango instance was still running.
  The second launch registered Ctrl+Space normally.
- `dango.log` records `dango.clipboard.history: could not register its hotkey`
  for Ctrl+Shift+V; another application holds that chord on this machine.
- The template error copy includes `(in <string>:1)` from the template engine,
  which the interface-copy spec would call an internal identifier on screen.
  Pre-existing text, untouched here.
- `Text::restore` in `src-tauri/src/text/mod.rs` returns early when nothing was
  on the clipboard before a selection read, so the `dango-selection-<uuid>`
  sentinel stays on the clipboard in that case. Observed here after a failed
  read; unrelated to this change.
