## 1. Take Ctrl+J/K off the selection

- [x] 1.1 Add `vimBindings={false}` to the `Command.Root` in `src/App.svelte`;
      verify by selecting the third result, pressing Ctrl+K, and seeing the
      panel open for the third result with the selection unmoved, and that
      Ctrl+J, Ctrl+N, and Ctrl+P leave the selection alone.
- [x] 1.2 Add `vimBindings={false}` to the `Command.Root` in
      `src/lib/ProtocolView.svelte`; verify the same in the clipboard history
      list, including Cmd+K on macOS.
- [x] 1.3 Run `npm run check` and confirm it passes.
- [x] 1.4 Add `src/lib/platform.ts` with the platform's modifier name and a
      shortcut formatter, and use it for the footer's action panel chord, the
      form submit chord, and the action panel's shortcut labels; verify in a
      headless run on macOS that the footer reads `⌘ K` and a Ctrl+X shortcut
      reads `⌘X`.
- [x] 1.5 Make the footer's primary label for a list view come from the items'
      first action rather than the fixed "Select"; verify the clipboard history
      footer reads "Paste to Active App".

## 2. Teach the text exchange to paste clipboard content

- [x] 2.1 In `src-tauri/src/text/mod.rs`, add `paste_content(&self, content:
      &Content)` to `TextTarget` and `TextExchange`: permission check, dismiss,
      yield, mark the write as Dango's own, write the content, paste, settle,
      with no save or restore; verify with new tests that a text and an image
      content each end up on the fake clipboard after the paste, that the
      launcher was dismissed exactly once, and that `insert` still restores.
- [x] 2.2 Verify with a test that a missing permission and a failed yield
      both return the error before the clipboard is written.
- [x] 2.3 Add `paste_content` to `FakeTarget` in
      `src-tauri/src/extensions/snippets/extension.rs` tests, recording the
      content; verify `cargo test` compiles and the snippets tests pass.

## 3. Give clipboard history a paste action and a primary-action preference

- [x] 3.1 Declare `PREF_PRIMARY_ACTION = "primary-action"` as a string
      preference with default `paste` in the clipboard manifest and add
      `ACTION_INSERT = "insert"`; verify the manifest test's preference count
      goes to 5 and `preference_declarations()` lists the new key.
- [x] 3.2 Extend `ClipboardExtension::new` to take a `Preferences` reader and
      an `Option<Arc<dyn TextTarget>>`, and build each item's actions as paste
      then copy then remove by default, copy then paste then remove when the
      preference reads `copy`, with any other value treated as `paste`; verify
      with unit tests over a memory preference store for all three values.
- [x] 3.3 Implement the `insert` action: `PermissionMissing` failure when there
      is no text target, otherwise `paste_content` on the entry's content,
      `Done` on success and `Failed` with the error's message otherwise; verify
      with tests using a fake target that a text entry and an image entry each
      reach the target and that a failing target reports its message.
- [x] 3.4 Wire it in `src-tauri/src/lib.rs`: build a second `Preferences` over
      the clipboard declarations and pass the managed `TextExchange`, or `None`
      when key injection is unavailable, into the extension; verify
      `cargo clippy -- -D warnings` is clean and the app starts with the history
      command listed.
- [x] 3.5 Document `primary-action` in `docs/config.example.jsonc` under the
      clipboard extension's preferences with the two values and the default;
      verify the file is still valid JSONC and reads clearly.

## 4. Verify on macOS

- [x] 4.1 With no preference set, open the history from an editor, press Enter
      on a text entry, and confirm the launcher hides, the text lands in the
      editor within 400ms, Cmd+V afterwards pastes the same entry again, and
      the history order is unchanged with no new entry.
- [x] 4.2 Paste an image entry into Preview or a rich editor and confirm the
      image lands and stays on the clipboard.
- [x] 4.3 Set `primary-action` to `copy` in `config.json` while Dango runs,
      reopen the history, press Enter, and confirm the entry is on the clipboard,
      the launcher hid, and nothing was pasted; open the panel with Cmd+K and
      confirm "Paste to Active App" is listed and works.
- [x] 4.4 Set `primary-action` to a nonsense value and confirm Enter pastes.
- [ ] 4.5 Revoke the Accessibility permission, press Enter on an entry, and
      confirm nothing is pasted, the launcher stays open with the permission
      message, and the clipboard is unchanged.
- [x] 4.6 Measure the time from Enter to the window hiding with
      `DANGO_MEASURE=1` and confirm it is under 100ms for both paste and copy.

## 5. Verify on Windows

- [ ] 5.1 Repeat 4.1 through 4.4 with Ctrl+K in place of Cmd+K, in Notepad and
      in a browser text field, and confirm the same results.
- [ ] 5.2 Confirm pasting needs no permission step and that the previous window
      is the foreground window before the paste lands.
- [ ] 5.3 Repeat 4.6 on Windows.
- [ ] 5.4 Confirm CI is green on `windows-latest` and `macos-latest`, including
      clippy, before the change is considered done.
