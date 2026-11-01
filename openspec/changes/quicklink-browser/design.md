## Context

A quicklink opens through `SystemOpener` (`src/opener.rs`), which calls
`open::that_detached`. Quicklinks live in `quicklinks.json` through the shared
`Records` store, which keeps unknown fields on a record. The create and edit
form is built in `form_tree` (`src/extensions/snippets/extension.rs`). The view
protocol already has a dropdown field with options.

## Goals / Non-Goals

**Goals:**

- One stable browser id per browser, the same in `quicklinks.json` on both
  machines, so a dotfiles copy works on either.
- A fallback that needs no detection code: try the browser, and on error open
  the default.

**Non-Goals:**

- Checking which browsers are installed before showing the form.

## Decisions

### Open with the `open` crate's `with_detached`, fall back to `that_detached`

`open::with_detached(url, app)` is already in the dependency tree, and it
reports a missing browser as an error on both platforms:

- Windows: the app name is looked up in the registry's App Paths keys (HKCU,
  then HKLM), then started. A name it cannot find fails to spawn. On the
  author's machine `firefox.exe`, `msedge.exe`, and `brave.exe` are registered
  there. Chrome registers `chrome.exe` there too, which still needs a check on a
  machine that has it.
- macOS: it runs `open -a "<App>" -- <url>` and waits for the exit code, which
  is not zero when no app has that name.

On an error, the opener calls `open::that_detached(url)`, as today. Only when
that also fails does the quicklink show the existing failure message.

Rejected:

- `webbrowser` crate: its `Browser` enum has no Edge or Brave, and on Windows it
  supports only `Browser::Default` (checked in 1.2.6, `src/windows.rs`).
- Our own `ShellExecuteExW` on Windows, or `NSWorkspace` with bundle ids on
  macOS: they do the same job as the `open` crate with more code. Bundle ids
  would survive a renamed `.app`, but the five browsers keep their names.
- Detecting installed browsers to filter the dropdown: more platform code for a
  list of five, and the fallback already covers a missing browser.

### A `Browser` enum with stable ids, app names behind the platform module

`Browser` (Chrome, Edge, Firefox, Brave, Safari) lives in feature code with its
id (`chrome`, `edge`, `firefox`, `brave`, `safari`) and its title. The app name
for each browser comes from one function in `platform/windows` and
`platform/macos`, returning `None` where the platform has no such browser
(Safari on Windows). That keeps `cfg` out of the quicklinks extension, as
AGENTS.md asks.

| id      | Windows       | macOS            |
|---------|---------------|------------------|
| chrome  | `chrome.exe`  | `Google Chrome`  |
| edge    | `msedge.exe`  | `Microsoft Edge` |
| firefox | `firefox.exe` | `Firefox`        |
| brave   | `brave.exe`   | `Brave Browser`  |
| safari  | none          | `Safari`         |

The dropdown lists the browsers whose app name is `Some` on this platform. An id
the app does not know (a typo in the file), or one with no app name here, opens
in the default browser.

### Storage: an optional `browser` field on the quicklink record

`Record` gains `browser: Option<String>`. The raw id is kept, not a parsed enum,
so a value this platform does not offer (`safari` on Windows) or does not know is
written back unchanged. Saving "Default browser" removes the field. Snippets
never set it.

### The edit form keeps a value it does not offer

The form's dropdown gets the stored id as its value. The renderer shows the
first option ("Default browser") when the value matches no option
(`render/form.rs`, `current` falls back to 0), and the state machine submits the
value it was given until the user picks something (`seeded_value`). So an edit on
Windows of a Safari quicklink saves `safari` again, with no new code. Picking
"Default browser" sends `default`, which clears the field.

### Interface text

Written with the `ux-writing` skill and against `interface-copy`. Product names
keep their own casing.

- Field label: "Browser"
- Options: "Default browser", "Google Chrome", "Microsoft Edge", "Firefox",
  "Brave", "Safari"

No new message: the fallback is silent, and a total failure keeps the existing
text.

### Contracts

No change to the extension manifest or the view protocol. The dropdown field
already exists.

## Risks / Trade-offs

- [The `open -a` app names on macOS are not checked yet] -> a hand check on
  the Mac opens one quicklink per browser.
- [A browser installed without an App Paths entry on Windows, for example a
  portable build] -> it opens in the default browser. Accepted.
- [A wrong macOS app name looks the same as a missing browser, and is hidden by
  the silent fallback] -> the fallback writes the error to the log.
- [`open -a` on macOS waits for Launch Services before returning] -> the same
  wait `that_detached` already has on macOS, so nothing changes.
