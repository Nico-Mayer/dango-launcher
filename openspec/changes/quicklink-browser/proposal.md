## Why

Today every quicklink opens in the default browser. Some links belong in another
browser, for example a work tool that only runs in one browser or uses its
logins. Picking the browser on the quicklink saves opening it by hand and pasting
the URL.

## What Changes

- A quicklink can name a browser. The create and edit form gets a "Browser"
  dropdown: "Default browser" first, then every browser this platform supports.
- Supported browsers: Google Chrome, Microsoft Edge, Firefox, and Brave on both
  platforms, plus Safari on macOS only.
- Opening a quicklink with a browser set opens it in that browser. If that
  browser is not installed, or the stored value is one this platform does not
  support, the quicklink opens in the default browser with no message.
- A quicklink without a browser behaves as today.
- `quicklinks.json` records gain an optional `browser` field.

## Non-goals

- Browser profiles (such as Chrome's `--profile-directory`).
- An "Open in…" action that picks a browser when you open the quicklink.
- Browsers beyond the five listed, or a free-text app name.
- Detecting which browsers are installed. The dropdown lists every supported
  browser either way.
- A browser for anything other than quicklinks (clipboard links, update notes).

## Platforms

Windows and macOS. Safari is offered only on macOS, because it does not exist on
Windows. A quicklink set to Safari still opens on Windows, in the default browser.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `quicklinks`: a quicklink can name the browser it opens in, with a silent
  fallback to the default browser. The create, edit, and open requirements
  change, and the restart requirement covers the browser too.

## Impact

- `src/extensions/snippets/` (`store.rs`, `extension.rs`): the record, the form,
  and the open path.
- `src/opener.rs`: open a URL in a named browser and fall back.
- `src/platform/windows`, `src/platform/macos`: each platform's app name per
  browser.
- `open` crate's `with_detached` opens the browser. `windows-registry`, already
  in `Cargo.lock`, becomes a direct Windows dependency to read the browser's
  path (see design.md).
- `quicklinks.json`: a new optional field. Old files load unchanged.
