## 1. Pin the standard

- [x] 1.1 Add the user-facing text convention to `Conventions` and the design
      rule to `rules.design` in `openspec/config.yaml`, using the wording in
      design.md; verify `openspec validate --change improve-user-facing-copy`
      passes and `openspec instructions design --change
      improve-user-facing-copy --json` lists the new rule.

## 2. Frontend strings

- [x] 2.1 In `src/App.svelte`, change the placeholder to "Search apps and
      commands", the empty row to `No results for “{query}”`, and replace the
      `working` boolean with a `workingTitle` string set from the confirmed
      item so the status line reads `Running {title}…`; verify by typing a
      nonsense query and seeing it quoted, and by invoking Clipboard History
      and seeing "Running Clipboard History…" until the list appears.
- [x] 2.2 In `src/lib/ProtocolView.svelte`, change the placeholder to
      "Search", the loading row to "Loading…", the filtered-empty row to
      `No results for “{query}”`, and the fallback to "Nothing to show";
      verify in the clipboard history list by filtering to nothing.
- [x] 2.3 In `src/lib/FormFields.svelte`, change "Asks for nothing" to
      "Nothing to fill in"; the inspection error's "This template can't be
      read: " lead-in comes from the backend `Display` in 3.9; verify by opening Create Snippet, typing
      `{{ name`, and seeing the prefixed message, then `{{ date }}` and seeing
      "Nothing to fill in".
- [x] 2.4 Run `npm run check` and confirm it passes.

## 3. Shared backend strings

- [x] 3.1 In `src-tauri/src/invocation.rs`, rewrite the two `InvocationError`
      messages and the panic message per design.md, update the "no good" test
      fixture only if it asserts on text; verify `cargo test invocation`
      passes.
- [x] 3.2 In `src-tauri/src/extension/mod.rs`, change the unavailable
      extension message to "That extension is turned off." and the default
      no-actions message to "That action isn't available."; verify
      `cargo test extension` passes.
- [x] 3.3 In `src-tauri/src/text/mod.rs`, rewrite the four `TextError`
      messages per design.md; verify `cargo test text` passes.
- [x] 3.4 In `src-tauri/src/platform/mod.rs`, rewrite `SystemError` and
      `WindowError` variant messages per design.md (`NoTarget`, `NotRunning`,
      `Unsupported`), leaving the `{0}` passthrough variants for the platform
      files to fill; verify `cargo build` succeeds.
- [x] 3.5 In `src-tauri/src/extensions/applications/mod.rs` and `index.rs`,
      change action titles to "Open", "Show in Finder" / "Show in File
      Explorer", "Copy path", and the three failure messages per design.md;
      verify `cargo test applications` passes and the footer reads "Open" on
      an application result.
- [x] 3.6 In `src-tauri/src/extensions/clipboard/extension.rs` and
      `history.rs`, change action titles to "Paste", "Copy", "Delete from
      history", the empty-state description, the unknown-action message, and
      the two `HistoryError` messages; update the test that checks action
      order by title if any; verify `cargo test clipboard` passes.
- [x] 3.7 In `src-tauri/src/extensions/snippets/store.rs`, rewrite every
      `RecordError` message per design.md and update the two tests asserting
      on "give it a URL" and "give it a valid URL"; verify `cargo test
      snippets::store` passes.
- [x] 3.8 In `src-tauri/src/extensions/snippets/extension.rs`, change action
      titles ("Paste", "Delete"), the body label "Text", the save titles, the
      `failure_tree` title per kind, and the no-opener and unknown-action
      messages; verify `cargo test snippets` passes.
- [x] 3.9 In `src-tauri/src/templates/mod.rs`, prefix `TemplateError::Parse`
      and `Render` displays per design.md; verify `cargo test templates`
      passes and the snippets tests still pass.
- [x] 3.10 In `src-tauri/src/extensions/system/mod.rs`, change the quit-list
      empty title to "No apps to quit" and the unknown-action message; verify
      `cargo test system` passes.
- [x] 3.11 Add a `Display` test to each of `InvocationError`, `TextError`,
      `SystemError`, `WindowError`, `RecordError`, `HistoryError`, and
      `TemplateError` that constructs every variant and asserts the text
      starts with an uppercase letter and ends with `.`; verify `cargo test`
      passes.
- [x] 3.12 Run `cargo clippy -- -D warnings` and confirm it is clean.

## 4. Tray

- [x] 4.1 In `src-tauri/src/lib.rs`, rewrite the notices, the launcher hotkey
      notice, `config_status_text`, and the "Toggle Dango" item per the tray
      table in design.md; verify by starting Dango with a broken
      `config.json` and seeing "Config file has an error, see dango.log", then
      fixing it and seeing "Config file loaded".

## 5. macOS platform strings

- [x] 5.1 In `src-tauri/src/platform/macos/window.rs`, rewrite
      `PERMISSION_MISSING`, the `Unreachable` message, and collapse the
      attribute-read failures into "Couldn't read the window's position. Try
      again." with the attribute and `AXError` code logged via `eprintln!`;
      verify `cargo test` passes on macOS.
- [x] 5.2 In `src-tauri/src/platform/macos/system.rs`, rewrite the lock,
      sleep, trash-count, and not-an-app messages per design.md, logging
      Finder's and `pmset`'s output instead of showing it; verify `cargo
      test` passes on macOS.
- [x] 5.3 In `src-tauri/src/platform/macos/text.rs`, replace the enigo and
      main-thread messages with "Couldn't send the keystroke. Try again." and
      log the detail; verify `cargo test` passes on macOS.

## 6. Windows platform strings

- [x] 6.1 In `src-tauri/src/platform/windows/window.rs`, rewrite the elevated,
      frame, display, and move-refused messages per design.md; verify by
      reading the diff against the table, since it cannot compile locally.
- [x] 6.2 In `src-tauri/src/platform/windows/system.rs`, rewrite the lock,
      sleep, recycle bin, not-an-app, and no-window messages per design.md,
      logging the Win32 error instead of showing it; verify by reading the
      diff against the table.
- [x] 6.3 In `src-tauri/src/platform/windows/text.rs`, rewrite the elevated,
      foreground, and enigo messages per design.md and log the enigo detail;
      verify by reading the diff against the table.
- [x] 6.4 In `src-tauri/src/platform/windows/apps.rs`, change the shell
      launch failure to "Couldn't open {name}. Try again."; verify by reading
      the diff.
- [x] 6.5 Push and confirm CI is green on `windows-latest`, including clippy
      and the new `Display` tests.

## 7. Verify on macOS

- [ ] 7.1 Walk every surface and check it against design.md: root placeholder
      and empty state, an application's action panel, the clipboard history
      list and panel, the snippet and quicklink lists, forms, and empty
      states, the quit-application list, and the empty-trash confirmation.
- [ ] 7.2 Revoke the Accessibility permission, paste a history entry, insert
      a snippet, and run Left Half; confirm each banner names the permission
      and System Settings, and contains no code.
- [ ] 7.3 Disable the snippets extension in `config.json` while its results
      are on screen and confirm the banner reads the "isn't available any
      more" message on Enter.
- [ ] 7.4 Break `snippets.json` on disk, open Search Snippets, and confirm
      the empty state reads "Couldn't read your snippets" with the JSON detail
      below; confirm the tray shows "snippets.json has an error, see
      dango.log".
- [ ] 7.5 Register Option+Space in another app, start Dango, and confirm the
      tray reads "Option+Space is in use, set launcher.hotkey".

## 8. Verify on Windows

- [x] 8.1 Repeat 7.1 on Windows, checking "Show in File Explorer", "Empty
      Recycle Bin", and Ctrl in place of Cmd.
- [x] 8.2 Run Left Half with an elevated window focused and confirm the banner
      reads the "running as administrator" message; repeat for pasting a
      history entry into the elevated window.
- [x] 8.3 Register Alt+Space in another app, start Dango, and confirm the tray
      reads "Alt+Space is in use, set launcher.hotkey".
- [x] 8.4 Repeat 7.3 and 7.4 on Windows.
- [x] 8.5 Confirm CI is green on both `windows-latest` and `macos-latest`
      before the change is considered done.
