## 1. Browser list and opener

- [x] 1.1 Add a `Browser` enum (Chrome, Edge, Firefox, Brave, Safari) with its stored id and form title, and a lookup from id; verify with unit tests that every id round-trips and an unknown id gives `None` (`cargo test`)
- [x] 1.2 Add the Windows app name per browser in `src/platform/windows` (`chrome.exe`, `msedge.exe`, `firefox.exe`, `brave.exe`, none for Safari); verify with a unit test that Safari has no app name and the others do (`cargo test` on Windows)
- [ ] 1.3 Add the macOS app name per browser in `src/platform/macos` (`Google Chrome`, `Microsoft Edge`, `Firefox`, `Brave Browser`, `Safari`); verify `cargo clippy --all-targets -- -D warnings` passes on macOS, or CI's macOS job when working from Windows
- [x] 1.4 Let `OpenUrl::open` take an optional browser id; `SystemOpener` tries `open::with_detached` with the platform's app name, logs the error and falls back to `open::that_detached` when it fails or there is no app name; verify with an opener test that an unknown id and a browser with no app name both take the default path (`cargo test opener`)

## 2. Quicklink record and form

- [x] 2.1 Add `browser: Option<String>` to `Record`, read from and written to the `browser` field of a quicklink; saving `default` or nothing removes the field; verify with store tests that a browser survives a reload, `default` clears it, and an id the platform does not offer is written back unchanged (`cargo test snippets`)
- [x] 2.2 Add the "Browser" dropdown to the quicklink form only: "Default browser", then each browser with an app name on this platform; the edit form passes the stored id as the value; verify with extension tests for the options on this platform, the selected value on edit, and that the snippet form has no such field (`cargo test snippets`)
- [x] 2.3 Pass the quicklink's browser to the opener on open, with and without a query; verify with `FakeOpener` tests that record the browser id for both paths and `None` for a quicklink without one (`cargo test snippets`)
- [ ] 2.4 UI test in `src/slint_shell/tests.rs`: create a quicklink, pick "Firefox" in the "Browser" dropdown, save, and confirm the stored record names `firefox` (`cargo test slint_shell`)

## 3. Hand checks on Windows

- [ ] 3.1 With a debug build up through the `dango-ui` MCP server, set three quicklinks to Edge, Firefox, and Brave, open each from root search, and confirm with `mise run shot` that each opens in its browser
- [ ] 3.2 Set a quicklink to Google Chrome (not installed here) and edit `quicklinks.json` to give another one `safari`; open both and confirm each opens in the default browser with no message, and that editing and saving the Safari one keeps `safari` in the file

## 4. Hand checks on macOS

- [ ] 4.1 Confirm `cargo test` passes on a Mac
- [ ] 4.2 Set one quicklink per browser that is installed, Safari included, open each from root search, and confirm each opens in its browser; this also checks the `open -a` app names
- [ ] 4.3 Set a quicklink to a browser that is not installed and confirm it opens in the default browser with no message

## 5. Wrap up

- [ ] 5.1 Run `mise run check` and `cargo test`; verify both pass
- [ ] 5.2 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context; it reports findings only and the author decides what to fix
