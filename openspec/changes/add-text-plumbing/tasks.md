## 1. Spike: the things that must be true before anything is built on them

The clipboard change was rescued by its spike overturning two design decisions
before group 5. The same three questions here are the ones that would be
expensive to answer late.

- [x] 1.1 Add `enigo`, `objc2-application-services`, and `minijinja` with the features design.md names, and verify `cargo build` and `cargo clippy -- -D warnings` pass on macOS with no Linux dependency pulled in
  - Builds and `cargo clippy --all-targets -- -D warnings` is clean. `cargo tree -i` finds no `x11rb`, `wayland-client`, `xkbcommon`, or `ashpd`, so nothing Linux entered the build.
  - One correction to the design's feature list: `minijinja` needs `serde` as well as `builtins` and `urlencode`. `Environment::new` is deprecated without it and the deprecation is a warning, which CI turns into a failure.
  - One thing the design got half right. It rejected `axuielement` for pulling a second Core Foundation stack, and `enigo` pulls one anyway: `core-foundation` 0.10 and `core-graphics` 0.25. The difference is real but narrower than the design implies: those types stay inside `enigo`, since its API is its own `Key` enum, whereas AX values would have crossed into our own AppKit code. Confirmed absent: `apple-cf` and `cocoa`.
- [ ] 1.2 On macOS, read `AXSelectedText` from the focused element of the frontmost application, and record which applications answer and which return nothing: a native app, a browser, an Electron app, and a terminal
- [ ] 1.3 On macOS, inject Cmd+V into another application after hiding the panel, and measure how long after the keystroke the clipboard can safely be restored, across a fast native app and a slow Electron one
- [ ] 1.4 On macOS, record how often Accessibility trust has to be re-granted across rebuilds of an unsigned debug binary, so the development cost is known rather than assumed
- [ ] 1.5 On Windows, confirm that waiting for the previous window to become foreground before injecting is reliable under foreground lock, and that a no-op message to the target after Ctrl+V returns only once the paste has been handled
- [ ] 1.6 On Windows, confirm the clipboard round trip reads a selection from a native app, a browser, an Electron app, and a terminal
- [x] 1.7 Verify `minijinja::Template::undeclared_variables` reports the names in the templates snippets will actually hold, including one with reserved names only and one with a repeated argument
  - Works exactly as the design needs. Reserved-only reports only reserved names, a repeated argument is reported once, and an unclosed placeholder fails to parse so it can be refused at save.
  - **A finding that contradicts the delimiter decision.** Single braces are safe, as the design argued: `fn main() { let x = Foo { a: 1 }; }` reports no placeholders. Doubled braces are not, and they are common in exactly the text an author stores as a snippet.
    - `printf("{{%d}}", x)` is a parse error, so the snippet is refused outright.
    - `runs-on: ${{ matrix.os }}` parses and reports an argument named `matrix`. A GitHub Actions snippet saves cleanly and then asks the wrong question every time it is used.
    - `<p>{{ user.name }}</p>` does the same with `user`, so Vue, Angular, and Handlebars snippets all misbehave silently.
  - Escapes exist and work: `{% raw %}...{% endraw %}` and `{{ '{{' }}` both parse to no placeholders. Both require editing text the user pasted in.
  - See 1.8. This needs a decision before group 2.
- [ ] 1.8 If any of the above does not work, revise design.md before building on it

## 2. The template engine

Pure Rust with no platform and no UI, so it can be finished and tested first.

- [ ] 2.1 Add the `templates` module wrapping `minijinja` with no loader, and verify a template with no placeholders renders back to itself, including one containing single braces
- [ ] 2.2 Resolve the reserved vocabulary (`clipboard`, `selection`, `date`, `uuid`, `cursor`, `query`) from a values source the caller supplies, with a test per name
- [ ] 2.3 Render a missing `clipboard` or `selection` as empty text rather than failing, with a test
- [ ] 2.4 Report a template's arguments as every referenced name that is not reserved, with tests for none, one, two, and a repeated name
- [ ] 2.5 Order reported arguments by first occurrence in the source, with a test that the order is stable across repeated calls
- [ ] 2.6 Render with supplied argument values, and render a blank argument as empty text, with tests
- [ ] 2.7 Resolve `cursor` to a caret offset and strip it from the output, honouring the first only, with tests for none, one, and two
- [ ] 2.8 Encode a value placed into a URL, with tests over spaces and over `&`, `?`, and `#`
- [ ] 2.9 Reject a template that cannot be parsed with a message naming the problem, with a test over an unclosed placeholder
- [ ] 2.10 Verify a 1,000-character template with 10 placeholders renders in under 5ms, with a test carrying the budget

## 3. The form submit path

Closes a `view-protocol` requirement that has been specified and unimplemented
since M1. Nothing else in this change works without it.

- [ ] 3.1 Widen `Extension::perform_action` to carry the submitted field values, update the three existing extensions to ignore them, and verify the existing test suite passes unchanged
- [ ] 3.2 Carry the values through `ExtensionHost::perform_action` and the `run_action` command, with a test that an extension receives what was submitted
- [ ] 3.3 Wire `ProtocolView`'s `onsubmit` to `run_action` with the form's primary action, replacing the empty function in `App.svelte`, and verify a form submit reaches the backend
- [ ] 3.4 Verify submitting a form that returns `ActionOutcome::Replaced` leaves the user on the replaced view rather than closing the launcher

## 4. Selection and paste behind traits

- [ ] 4.1 Define `SelectionSource` and `TextInjector` traits covering reading the selection, inserting text, placing the caret, and reporting a missing permission, with fakes for testing
- [ ] 4.2 Implement the shared clipboard round trip for reading a selection: save, copy, read, restore, tested against a fake clipboard and a fake injector
- [ ] 4.3 Implement the shared insertion path: save the clipboard, write the text, send the paste shortcut, restore, tested the same way
- [ ] 4.4 Abandon the restore when the clipboard changed underneath the operation, with a test that the user's newer content is left alone
- [ ] 4.5 Move the caret by counting characters after the caret offset, with a test over text containing multi-byte characters
- [ ] 4.6 Report a failed insertion without disturbing the clipboard, with a test
- [ ] 4.7 Change the clipboard watcher's own-write suppression from one slot to a short queue, and verify with tests that both writes of a paste are suppressed, that a restore does not reorder the history, and that the user genuinely re-copying the same content is still recorded

## 5. Platform: macOS

- [ ] 5.1 Read the selection through `AXUIElementCreateSystemWide`, the focused element, and `AXSelectedText`, verified by hand against the applications from 1.2
- [ ] 5.2 Fall back to the clipboard round trip when the accessibility API returns nothing, verified against an application from 1.2 that declined to answer
- [ ] 5.3 Inject Cmd+V through `enigo` with `independent_of_keyboard_state` on and the permission prompt off, verified by pasting into another application
- [ ] 5.4 Hide the panel and wait for it to resign key before injecting, verified by confirming the text never lands in the launcher's own field
- [ ] 5.5 Restore the clipboard after the delay measured in 1.3, verified by checking the clipboard's contents after a paste into a slow application
- [ ] 5.6 Report the missing Accessibility permission through `AXIsProcessTrusted` and offer the prompt as an action, verified by revoking the permission and pasting
- [ ] 5.7 Verify that `restore_previous_focus` remains the correct no-op here, by confirming the target application was never deactivated

## 6. Platform: Windows

Cannot be compiled on macOS. These land as code plus confirmations the author
closes on the Windows machine, the way the clipboard change did.

- [ ] 6.1 Read the selection through the shared clipboard round trip, verified by hand against the applications from 1.6
- [ ] 6.2 Inject Ctrl+V through `enigo` with `windows_dw_extra_info` set so the injected events are identifiable, verified by pasting into another application
- [ ] 6.3 Hide the launcher, call `restore_previous_focus`, and wait for `GetForegroundWindow` to return the target before injecting, verified by pasting immediately after activation
- [ ] 6.4 Fail with a message when the previous window never becomes foreground, verified by holding foreground elsewhere
- [ ] 6.5 Wait for the target window to answer a no-op message after Ctrl+V before restoring the clipboard, verified by checking the clipboard after pasting into a .NET application
- [ ] 6.6 Report the elevated-window ceiling as a clear failure rather than a silent one, verified by pasting into an elevated window
- [ ] 6.7 Check symbol names and module paths against the vendored crate source before pushing, and verify CI's `cargo clippy -- -D warnings` passes on `windows-latest`

## 7. Snippets

- [ ] 7.1 Add the migration for the snippets table following the syncable convention, and verify it applies to an existing database without touching the other tables
- [ ] 7.2 Declare the manifest with its create and search commands and its root provider, and test that it validates
- [ ] 7.3 Store, update, and soft-delete a snippet, with tests including that a removed snippet stays removed across a reopen
- [ ] 7.4 Refuse a snippet with an empty name or template, or a template that will not parse, with tests and a message the form shows
- [ ] 7.5 Contribute snippets as root items matched on name, with a test, and verify the provider answers within 50ms with 500 snippets stored
- [ ] 7.6 Build the create and edit forms on the `Form` view kind, reusing the submit path from group 3, verified by creating a snippet from the launcher
- [ ] 7.7 Insert a snippet's rendered text on confirm, with a test over a template needing no arguments
- [ ] 7.8 Push an argument form when the template needs arguments, in template order, and insert on submit, with a test
- [ ] 7.9 Leave nothing inserted and the clipboard untouched when the argument form is dismissed, with a test
- [ ] 7.10 Add the copy action as an alternative to inserting, and verify the copied text is recorded in the clipboard history as the user's own copy
- [ ] 7.11 Add the remove action leaving the list open, with a test

## 8. Quicklinks

- [ ] 8.1 Add the migration for the quicklinks table following the syncable convention, and verify it applies to an existing database without touching the other tables
- [ ] 8.2 Declare the manifest with its create and search commands and its root provider, and test that it validates
- [ ] 8.3 Store, update, and soft-delete a quicklink, with tests including that a removed quicklink stays removed across a reopen
- [ ] 8.4 Refuse a quicklink with an empty name or URL, or a template that cannot form a valid URL, with tests
- [ ] 8.5 Contribute quicklinks as root items matched on name, with a test, and verify the provider answers within 50ms with 500 quicklinks stored
- [ ] 8.6 Open the rendered URL in the default browser on confirm, with a test over the rendering and a check by hand that it opens
- [ ] 8.7 Push a query form when the template uses `query`, and open on submit with the query encoded, with tests over spaces and reserved characters
- [ ] 8.8 Resolve `clipboard` and `selection` in a quicklink without asking the user, with a test
- [ ] 8.9 Report a URL that could not be opened without closing the launcher, with a test
- [ ] 8.10 Add the copy-URL action, with a test

## 9. Verification

- [ ] 9.1 Walk every scenario in the four spec files on macOS
- [ ] 9.2 Walk every scenario in the four spec files on Windows
- [ ] 9.3 Confirm on both platforms that the user's clipboard is identical before and after a paste, for text and for an image
- [ ] 9.4 Confirm on both platforms that no paste, restore, or selection capture appears in the clipboard history or reorders it, while the watcher is running
- [ ] 9.5 Confirm on both platforms that a snippet with a caret position leaves the caret where the template declared it, in at least two applications
- [ ] 9.6 Confirm on both platforms that activation still meets the 80ms budget on a release build with both new extensions enabled
- [ ] 9.7 Confirm on both platforms that text appears in the target application within 400ms of confirming, measured over repeated pastes
- [ ] 9.8 Confirm on macOS that revoking the Accessibility permission produces the explained failure and the prompt action, and that granting it restores normal behaviour without a restart
- [ ] 9.9 Confirm on Windows that pasting into an elevated window fails visibly and leaves the clipboard alone
- [ ] 9.10 Confirm on both platforms that snippets and quicklinks survive a restart with their names and templates intact
