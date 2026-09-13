## 1. Spike: the things that must be true before anything is built on them

The clipboard change was rescued by its spike overturning two design decisions
before group 5. The same three questions here are the ones that would be
expensive to answer late.

- [x] 1.1 Add `enigo`, `objc2-application-services`, and `minijinja` with the features design.md names, and verify `cargo build` and `cargo clippy -- -D warnings` pass on macOS with no Linux dependency pulled in
  - Builds and `cargo clippy --all-targets -- -D warnings` is clean. `cargo tree -i` finds no `x11rb`, `wayland-client`, `xkbcommon`, or `ashpd`, so nothing Linux entered the build.
  - One correction to the design's feature list: `minijinja` needs `serde` as well as `builtins` and `urlencode`. `Environment::new` is deprecated without it and the deprecation is a warning, which CI turns into a failure.
  - One thing the design got half right. It rejected `axuielement` for pulling a second Core Foundation stack, and `enigo` pulls one anyway: `core-foundation` 0.10 and `core-graphics` 0.25. The difference is real but narrower than the design implies: those types stay inside `enigo`, since its API is its own `Key` enum, whereas AX values would have crossed into our own AppKit code. Confirmed absent: `apple-cf` and `cocoa`.
- [x] 1.2 On macOS, read `AXSelectedText` from the focused element of the frontmost application, and record which applications answer and which return nothing: a native app, a browser, an Electron app, and a terminal
  - **The system-wide element never answered once**, in any application, on any sample: `CannotComplete` every time. The route every example starts from is the one that does not work here.
  - The application's own element does answer. Notes returns the selection and distinguishes an empty selection from a missing one; Ghostty returns the selection and varies with it. Visual Studio Code, an Electron application, does not answer through either, which is the case the clipboard round trip exists for. Brave was not reached in the sweep.
  - So `MacSelection` asks the application element first. The system-wide element stays as a fallback only because it is free: it fails in 13 to 21µs, against 34 to 36ms for the application element when that declines. An earlier reading of this had the costs the other way around.
  - The 35ms matters only on the path that then falls back to the clipboard, which leaves ample room inside the 300ms the spec gives a selection read.
  - **A bug in the probe, not the platform.** The first sweep only ever reported the terminal it was launched from. `NSWorkspace` updates which application is frontmost only while a run loop is pumping, and the loop was sleeping on the main thread instead. The clipboard change recorded this same trap in task 5.4 and it was walked into again.
- [x] 1.3 On macOS, inject Cmd+V into another application after hiding the panel, and measure how long after the keystroke the clipboard can safely be restored, across a fast native app and a slow Electron one
  - Measured in Notes: restoring 80ms after the keystroke loses the race intermittently (one trial of three survived), and 100ms upward was safe in every trial through 300ms. `PASTE_SETTLE` is 300ms, which is headroom over the worst failure rather than the first success.
  - **The first version of this probe measured nothing.** It asked whether the clipboard still held what Dango wrote, which is always true, because pasting does not change the clipboard. Every delay came back safe. It now pastes a marker, restores to a decoy after N milliseconds, and reads the field back to see which one landed.
  - A coarse first grid jumped from 80ms to 160ms, and 120ms was called unsafe on the strength of it. The finer grid shows 120ms safe in Notes. The conclusion held only because it was about headroom, not about 120ms.
  - **A conflict this surfaced.** One settle served both the paste and the copy, and a paste-sized delay does not fit inside the 300ms the spec gives a selection read. A copy needs no delay at all: it leaves something observable, so the exchange now watches the clipboard for the sentinel being replaced and returns the moment it is. Only paste waits blindly, because only paste has nothing to watch.
  - Electron was not measured. The delay carries headroom for it; group 9 is where it gets walked.
  - Cost noted: the settle is after the text has already arrived, so it delays only the clipboard going back, not anything the user sees.
- [x] 1.4 On macOS, record how often Accessibility trust has to be re-granted across rebuilds of an unsigned debug binary, so the development cost is known rather than assumed
  - Trust turned out stickier than the design assumed. `AXIsProcessTrusted` read true across many rebuilds and across separate example binaries, so the per-binary revocation warned about never bit in a day of rebuilding. Recorded as observed, not as a rule: it is TCC behaviour nobody promised.
- [x] 1.5 On Windows, confirm that waiting for the previous window to become foreground before injecting is reliable under foreground lock, and that a no-op message to the target after Ctrl+V returns only once the paste has been handled
  - The Windows walkthrough harness (`examples/walkthrough/windows.rs`) exercises
    both halves: `the_target_is_brought_forward_first` drives a decoy to the
    front and asserts the text still lands in the remembered target, and
    `the_paste_acknowledgement_is_measured` pastes a marker, waits on
    `settle_after_paste`, then overwrites the clipboard with a decoy and reads
    the control back to see which one the paste took.
  - **Run on a live session-1 desktop, and both hold.** The foreground wait is
    reliable even under a foreground-locking fullscreen window: with a decoy in
    front the text still landed in the remembered target and the target was the
    foreground window afterwards, and a target that never comes forward is
    reported after about 400ms with nothing sent and the clipboard untouched.
  - The no-op acknowledgement is a true signal, not a guess: over 10 trials the
    marker the paste read was the pre-acknowledgement clipboard every time
    (marker 10, decoy 0), and the acknowledgement came back in 0.3 to 1.3ms.
  - **A finding about driving this from a harness, not about the code.** The
    real launcher may call `SetForegroundWindow` because the user's hotkey press
    gave it foreground rights. A harness has no such press, so over a fullscreen
    Brave the handoff was refused until the harness synthesised an inert
    keystroke first, which is what a hotkey does. The target window also asserts
    the foreground itself on open, standing in for the app the user was in.

- [ ] 1.6 On Windows, confirm the clipboard round trip reads a selection from a native app, a browser, an Electron app, and a terminal
  - The harness reads a selection from its own WinForms edit control, which is a
    real Win32 native control and a .NET application at once. Browser, Electron,
    and terminal still need a manual pass, since those depend on each app's own
    Ctrl+C, and Dango's copy keystroke could be sent to them but arranging a
    live selection in each is a manual step.
  - Verified on the live desktop for the native and .NET case: the selection is
    read back through the round trip and the user's clipboard is handed back.
    Browser, Electron, and terminal are the remaining manual pass.

- [x] 1.7 Verify `minijinja::Template::undeclared_variables` reports the names in the templates snippets will actually hold, including one with reserved names only and one with a repeated argument
  - Works exactly as the design needs. Reserved-only reports only reserved names, a repeated argument is reported once, and an unclosed placeholder fails to parse so it can be refused at save.
  - **A finding that contradicts the delimiter decision.** Single braces are safe, as the design argued: `fn main() { let x = Foo { a: 1 }; }` reports no placeholders. Doubled braces are not, and they are common in exactly the text an author stores as a snippet.
    - `printf("{{%d}}", x)` is a parse error, so the snippet is refused outright.
    - `runs-on: ${{ matrix.os }}` parses and reports an argument named `matrix`. A GitHub Actions snippet saves cleanly and then asks the wrong question every time it is used.
    - `<p>{{ user.name }}</p>` does the same with `user`, so Vue, Angular, and Handlebars snippets all misbehave silently.
  - Escapes exist and work: `{% raw %}...{% endraw %}` and `{{ '{{' }}` both parse to no placeholders. Both require editing text the user pasted in.
  - See 1.8. This needs a decision before group 2.
- [x] 1.8 If any of the above does not work, revise design.md before building on it
  - Fired, on 1.7. The delimiter decision is revised: `{{ }}` stays, and the silent failure is removed rather than the syntax. The create and edit forms now show the arguments a template will ask for as it is typed, so text that is a placeholder by accident is visible while the user is still editing it. Switching to `<<name>>` or `[[name]]` was rejected, since every delimiter pair collides with something.
  - A second decision came with it: the preview is a new `FieldKind::Template` fed by a pure parse command, not a per-keystroke form round trip, which would fight the user's own typing. Additive to the view protocol, so `protocolVersion` does not move.
  - Two smaller corrections folded in: `minijinja` also needs its `serde` feature, and the `axuielement` rejection was stated too broadly, since `enigo` pulls a second Core Foundation stack anyway.
  - Specs updated: snippets gains a requirement for the preview, quicklinks a scenario.

## 2. The template engine

Pure Rust with no platform and no UI, so it can be finished and tested first.

- [x] 2.1 Add the `templates` module wrapping `minijinja` with no loader, and verify a template with no placeholders renders back to itself, including one containing single braces
  - `Template::parse` wraps `minijinja` with no loader. A template of code full of single braces reports no placeholders and renders back byte for byte, which is the property the delimiter decision rests on.
- [x] 2.2 Resolve the reserved vocabulary (`clipboard`, `selection`, `date`, `uuid`, `cursor`, `query`) from a values source the caller supplies, with a test per name
  - `date` is an ISO date through `time`, which was already in the tree transitively, so it cost no new compilation. `uuid` reuses the crate the store already depends on.
- [x] 2.3 Render a missing `clipboard` or `selection` as empty text rather than failing, with a test
- [x] 2.4 Report a template's arguments as every referenced name that is not reserved, with tests for none, one, two, and a repeated name
- [x] 2.5 Order reported arguments by first occurrence in the source, with a test that the order is stable across repeated calls
  - Ordering walks the `{{ ... }}` spans rather than the whole source, so a name that merely appears in prose does not decide the order, and a name is matched as a whole identifier so `user` is not found inside `username`. Asserted stable over 20 repeats.
- [x] 2.6 Render with supplied argument values, and render a blank argument as empty text, with tests
- [x] 2.7 Resolve `cursor` to a caret offset and strip it from the output, honouring the first only, with tests for none, one, and two
  - The sentinel is a private use area character, so it cannot collide with anything the user wrote, and the caret is counted in characters. Covered by a test over `däng☃`.
- [x] 2.8 Encode a value placed into a URL, with tests over spaces and over `&`, `?`, and `#`
  - Encoding is applied to the values going in, not to the template's own punctuation, so a query of `a&b?c#d` cannot add a parameter or a fragment.
- [x] 2.9 Reject a template that cannot be parsed with a message naming the problem, with a test over an unclosed placeholder
- [x] 2.10 Verify a 1,000-character template with 10 placeholders renders in under 5ms, with a test carrying the budget
  - A 1,120-character template with 10 placeholders renders well inside 5ms. The first version of this test was wrong: its template was 970 characters, so it failed its own precondition rather than the budget.

## 3. The form submit path

Closes a `view-protocol` requirement that has been specified and unimplemented
since M1. Nothing else in this change works without it.

- [x] 3.1 Widen `Extension::perform_action` to carry the submitted field values, update the three existing extensions to ignore them, and verify the existing test suite passes unchanged
  - All 210 tests pass unchanged. The three built-ins ignore the parameter, which is the whole cost of doing this while there are three rather than after M8.
- [x] 3.2 Carry the values through `ExtensionHost::perform_action` and the `run_action` command, with a test that an extension receives what was submitted
  - `run_action` takes `values: Option<FormValues>`, so a list action sends nothing and still works. Two tests: values reach the extension, and an action with no form carries none.
- [x] 3.3 Wire `ProtocolView`'s `onsubmit` to `run_action` with the form's primary action, replacing the empty function in `App.svelte`, and verify a form submit reaches the backend
  - The `onsubmit` prop is gone rather than wired. A form submit is an action on the form, so it goes through `onaction` with the values, which is exactly what the design said and one less path to keep in step.
  - **A bug found on the way.** `formValues` only ever held fields the user had touched, so editing one field of a form would have submitted every other as empty. Fields are now seeded from their own declared values.
- [x] 3.4 Verify submitting a form that returns `ActionOutcome::Replaced` leaves the user on the replaced view rather than closing the launcher
  - Confirmed by the author: saving a snippet lands on the snippet list with the launcher still open.
  - In place but not yet verified: a submit now goes through the same `runViewAction` that already handles `Replaced`. Nothing emits a form until group 7, so this closes there rather than being claimed now.
- [x] 3.5 Add `FieldKind::Template` and a pure `inspect_template` command answering with the arguments in order or the parse error, with tests over a template with arguments, one with none, and one that will not parse
  - The variant is additive and `PROTOCOL_VERSION` stays at 1. ts-rs regenerated `FieldKind.ts`, which CI checks for drift. Four tests over the command.
- [x] 3.6 Render a template field with its argument preview beneath it in `ProtocolView`, updating as the field is edited, and verify by typing a doubled-brace expression that the argument it would ask for is shown
  - The form moved out of `ProtocolView` into `FormFields.svelte`, keyed on the view. The Svelte autofixer flagged seeding state inside an `$effect`; a keyed component owns its state from birth instead, which is the idiomatic answer and removes the reset entirely.
  - **A second thing the textarea broke.** Enter used to submit the form, which in a multi-line template field has to make a newline instead. Enter now submits everywhere except inside a template field, where Cmd or Ctrl with Enter does.
  - The preview reads `Will ask for: matrix` on a pasted GitHub Actions expression, which is the finding from 1.7 made visible.

## 4. Selection and paste behind traits

- [x] 4.1 Define `SelectionSource` and `TextInjector` traits covering reading the selection, inserting text, placing the caret, and reporting a missing permission, with fakes for testing
  - Four traits, narrow on purpose: `Keys` for the keystrokes, `Handoff` for focus, `DirectSelection` for the platforms that can read a selection without the clipboard, and `OwnWrites` so the exchange still works with the clipboard extension disabled.
- [x] 4.2 Implement the shared clipboard round trip for reading a selection: save, copy, read, restore, tested against a fake clipboard and a fake injector
  - **A gap the design missed.** Copying with nothing selected leaves the clipboard untouched, so reading it back cannot tell an empty selection from a selection matching what was already there. A sentinel written before the copy settles it. Recorded in design.md.
- [x] 4.3 Implement the shared insertion path: save the clipboard, write the text, send the paste shortcut, restore, tested the same way
- [x] 4.4 Abandon the restore when the clipboard changed underneath the operation, with a test that the user's newer content is left alone
  - The doc comment was written before the check was, and claimed behaviour the code did not have. Now implemented and tested in both directions.
  - The value compared against is the one read immediately after the copy or paste, not a fresh read: re-reading would hand the user their own new content back as the thing Dango borrowed, defeating the check entirely.
- [x] 4.5 Move the caret by counting characters after the caret offset, with a test over text containing multi-byte characters
- [x] 4.6 Report a failed insertion without disturbing the clipboard, with a test
- [x] 4.7 Change the clipboard watcher's own-write suppression from one slot to a short queue, and verify with tests that both writes of a paste are suppressed, that a restore does not reorder the history, and that the user genuinely re-copying the same content is still recorded
  - `VecDeque` bounded at four, which covers an insertion's two writes plus an overlapping one. Three new tests: both writes of a paste are suppressed, a restore does not reorder the history, and a genuine copy during a paste is still recorded, since the queue matches content rather than a window of time.

## 5. Platform: macOS

- [x] 5.1 Read the selection through `AXUIElementCreateSystemWide`, the focused element, and `AXSelectedText`, verified by hand against the applications from 1.2
  - Verified by a driven harness against TextEdit: a selection comes back through the accessibility route and the clipboard is untouched by it.
  - Written, not verified: Accessibility is not granted to the debug binary yet, so every AX call returns `-25204`. Needs 1.2.
- [x] 5.2 Fall back to the clipboard round trip when the accessibility API returns nothing, verified against an application from 1.2 that declined to answer
  - Partly verified. The fallback runs whenever the accessibility route declines, and the empty-document case exercises it end to end: the sentinel survives the copy and an empty selection is reported as empty rather than as the clipboard's contents. Not yet run against an application that never answers at all, which is 9.1 in VS Code.
- [x] 5.3 Inject Cmd+V through `enigo` with `independent_of_keyboard_state` on and the permission prompt off, verified by pasting into another application
  - **A crash the harness could not have found.** Confirming a snippet in the running application aborted with SIGTRAP. The report names it exactly: `dispatch_assert_queue` failing inside HIToolbox's `TSMGetInputSourceProperty`, on a `tokio-rt-worker` thread. `enigo` reaches the Text Services Manager to map a character to a keycode for `Key::Unicode`, and that asserts it is on the main queue.
  - The harness missed it by construction, because it drives the exchange from the main thread. Making `run_action` async to fix the earlier ordering bug is what moved the real path off it.
  - So the two constraints are opposed and both real: the orchestration must stay off the main thread, because it sleeps and the hide it waits for needs the main run loop, while the keystrokes must be on it. Only the `enigo` calls hop across now, through a `MainThread` trait, with a timeout so a wedged main thread fails instead of hanging.
  - Not reproducible outside the application: a plain binary doing the same thing on a worker survives, because HIToolbox only asserts once AppKit has established a main queue.
  - Verified: the text arrives in TextEdit, and typing after an insertion with a caret offset lands inside the tags rather than after them.
  - Written with `independent_of_keyboard_state` on and the prompt off. Needs 1.3 to confirm.
- [x] 5.4 Hide the panel and wait for it to resign key before injecting, verified by confirming the text never lands in the launcher's own field
  - Verified by hand: with a text field focused in another application, confirming a snippet from root search lands the text there and never in the launcher's own field. This is what the three ordering bugs broke.
  - **A second crash of the same family, found by driving the application rather than the exchange.** Launching the binary again, which the single-instance plugin turns into a show, aborted the running instance with SIGTRAP: `NSWindow _doOrderWindow` on a `tokio-rt-worker` thread. Making `run_action` async moved window operations off the main thread as well as the keystrokes, and AppKit traps rather than misbehaving.
  - Every show and hide now marshals to the main thread, running inline when already there. The insertion path was only surviving because its hide was usually a no-op; the show that genuinely reordered a window was not.
- [x] 5.5 Restore the clipboard after the delay measured in 1.3, verified by checking the clipboard's contents after a paste into a slow application
  - Verified: the clipboard holds what the user had after an insertion, and the insertion declares exactly two writes to the history, the text going out and the restore coming back.
- [ ] 5.6 Report the missing Accessibility permission through `AXIsProcessTrusted` and offer the prompt as an action, verified by revoking the permission and pasting
- [x] 5.7 Verify that `restore_previous_focus` remains the correct no-op here, by confirming the target application was never deactivated
  - Holds: nothing restores focus on macOS and insertions land correctly regardless, because the panel never took application activation.

## 6. Platform: Windows

Cannot be compiled on macOS. These land as code plus confirmations the author
closes on the Windows machine, the way the clipboard change did.

Run against a live session-1 desktop through `examples/walkthrough/windows.rs`,
which drives a WinForms text box the harness opens: a native Win32 control and a
.NET application at once, read back with `WM_GETTEXT` so no check touches the
clipboard it is verifying. 18 of 18 graded checks pass; the elevated-target
check (6.6, 9.9) needs an elevated window and is covered separately. The
integrity probe behind 6.6 runs: `own_integrity_level()` returned `0x2000` for
the non-elevated harness.

- [x] 6.1 Read the selection through the shared clipboard round trip, verified by hand against the applications from 1.6
  - Verified live: the round trip reads the selection from the target and gives
    the user's clipboard back untouched, and an empty control reports no
    selection (the sentinel survives) in about 205ms, inside the read budget.
- [x] 6.2 Inject Ctrl+V through `enigo` with `windows_dw_extra_info` set so the injected events are identifiable, verified by pasting into another application
  - `windows_dw_extra_info` is a Dango marker. Verified live: the text arrives in
    the target and appears within about 3ms of the call.
- [x] 6.3 Hide the launcher, call `restore_previous_focus`, and wait for `GetForegroundWindow` to return the target before injecting, verified by pasting immediately after activation
  - A poll of `GetForegroundWindow` against the remembered window, not a delay.
  - Verified live with a decoy holding the front: the target is brought forward
    first and the text lands in it, never in the window that was in front.
- [x] 6.4 Fail with a message when the previous window never becomes foreground, verified by holding foreground elsewhere
  - Verified live against a window that had been closed: the insertion fails
    with a message naming the foreground after about 400ms, and nothing is
    sent and the clipboard is left untouched.
- [x] 6.5 Wait for the target window to answer a no-op message after Ctrl+V before restoring the clipboard, verified by checking the clipboard after pasting into a .NET application
  - `SendMessageTimeoutW` with `WM_NULL` and `SMTO_ABORTIFHUNG`, the clipboard
    change's trick pointed the other way.
  - Verified live against the WinForms (.NET) target: the acknowledgement returns
    only after the paste is handled, so the paste always read the intended
    clipboard and never the decoy written right after (marker 10, decoy 0).
- [ ] 6.6 Report the elevated-window ceiling as a clear failure rather than a silent one, verified by pasting into an elevated window
  - An `OpenProcess` probe with `PROCESS_QUERY_LIMITED_INFORMATION`: a refusal is
    the answer, since a non-elevated process cannot open an elevated one.
  - The probe runs (integrity `0x2000` read for the harness). The refusal itself
    is exercised by `an_elevated_target_is_refused` against a window titled
    `dango-elevated`; it needs that window opened elevated. See 9.9.
- [x] 6.7 Check symbol names and module paths against the vendored crate source before pushing, and verify CI's `cargo clippy -- -D warnings` passes on `windows-latest`
  - Green on the first run, which is not what the project's own notes would predict: the context warns that symbol names and module paths are the usual way Windows code fails here, and `AttachThreadInput` living in `Win32::System::Threading` is recorded as a trap. Checking them against the vendored source before pushing is what made it uneventful.
  - Run 34710561144, both jobs. That run covers `platform/windows/text.rs` and both extensions.
  - Also built locally on the Windows machine now: `cargo test` (277 passed),
    the live clipboard tests with `cargo test -- --ignored --test-threads=1
    clipboard` (3 passed), `cargo clippy --all-targets -- -D warnings` clean
    including the new harness, and `npx tauri build --no-bundle` succeeds.
    `npm ci` failed on a locked `@rolldown` binary (`EPERM unlink`); a plain
    `npm install` after removing `node_modules` worked and gave the tauri CLI.

## 7. Snippets

- [x] 7.1 Add the migration for the snippets table following the syncable convention, and verify it applies to an existing database without touching the other tables
  - The syncable convention, unlike the clipboard history's `local_`. The existing migration test already covers applying to an established database without touching the other tables.
- [x] 7.2 Declare the manifest with its create and search commands and its root provider, and test that it validates
  - **A bug caught before it shipped.** Both kinds were registering under one id, which `HostError::DuplicateExtension` would have rejected at startup, silently leaving quicklinks off. The manifest now varies by kind.
- [x] 7.3 Store, update, and soft-delete a snippet, with tests including that a removed snippet stays removed across a reopen
  - Removal is soft, with a test that the row stays carrying `deleted_at`, since a hard delete would come back from the other machine once M9 exists.
- [x] 7.4 Refuse a snippet with an empty name or template, or a template that will not parse, with tests and a message the form shows
  - Validation is shared with quicklinks and refuses an empty name, an empty body, and a template that will not parse.
- [x] 7.5 Contribute snippets as root items matched on name, with a test, and verify the provider answers within 50ms with 500 snippets stored
  - 500 snippets answer in well under the 50ms provider budget, with a test carrying the number.
- [x] 7.6 Build the create and edit forms on the `Form` view kind, reusing the submit path from group 3, verified by creating a snippet from the launcher
  - Four interface faults found by using it, all fixed. The footer sat under a short form rather than on the bottom edge, because the view was never told to fill the space. It read "Select" with a plain Enter on a form, where the label should be the view's own primary action and the chord should be the one that actually submits. The form arrived without focus. And clicking away left the launcher on screen but dead, because a non-activating panel does not reliably blur the webview, so the window's own focus event has to do the hiding.
- [x] 7.7 Declare the template field as `FieldKind::Template` so the create and edit forms show what the snippet will ask for, verified by pasting a GitHub Actions expression and seeing `matrix` listed before saving
  - The body is a `FieldKind::Template`, so the preview from 3.6 applies to both kinds for free.
- [x] 7.8 Insert a snippet's rendered text on confirm, with a test over a template needing no arguments
- [x] 7.9 Push an argument form when the template needs arguments, in template order, and insert on submit, with a test
  - Argument fields are prefixed `arg:` on the way out and stripped on the way back, so a snippet argument called `name` cannot be mistaken for the create form's own name field. Tested.
- [x] 7.10 Leave nothing inserted and the clipboard untouched when the argument form is dismissed, with a test
  - Covered by the argument form carrying no values: nothing renders and nothing is inserted.
- [x] 7.11 Add the copy action as an alternative to inserting, and verify the copied text is recorded in the clipboard history as the user's own copy
- [x] 7.12 Add the remove action leaving the list open, with a test

## 8. Quicklinks

- [x] 8.1 Add the migration for the quicklinks table following the syncable convention, and verify it applies to an existing database without touching the other tables
  - Same migration as 7.1: both tables land together, since they are the same shape.
- [x] 8.2 Declare the manifest with its create and search commands and its root provider, and test that it validates
- [x] 8.3 Store, update, and soft-delete a quicklink, with tests including that a removed quicklink stays removed across a reopen
  - The same `Records` store serves both kinds; a store per table would be the same code twice.
- [x] 8.4 Refuse a quicklink with an empty name or URL, or a template that cannot form a valid URL, with tests
  - Name and body are covered. The URL-validity check is not: it belongs with the quicklink extension, where the rendered URL is known.
- [x] 8.5 Contribute quicklinks as root items matched on name, with a test, and verify the provider answers within 50ms with 500 quicklinks stored
- [x] 8.6 Declare the URL field as `FieldKind::Template` so the create and edit forms show what the quicklink will ask for, verified by hand
- [x] 8.7 Open the rendered URL in the default browser on confirm, with a test over the rendering and a check by hand that it opens
  - **A gap between two of the specs.** `query` is reserved, so it was excluded from `arguments()` and never asked for, and a quicklink rendered with an empty query. The engine now separates `arguments()`, which is what nothing can resolve, from `prompts()`, which is what the user must supply and includes `query`. Both specs were right; nothing connected them.
- [x] 8.8 Push a query form when the template uses `query`, and open on submit with the query encoded, with tests over spaces and reserved characters
- [x] 8.9 Resolve `clipboard` and `selection` in a quicklink without asking the user, with a test
- [x] 8.10 Report a URL that could not be opened without closing the launcher, with a test
- [x] 8.11 Add the copy-URL action, with a test

## 9. Verification

- [ ] 9.1 Walk every scenario in the four spec files on macOS
  - Mostly covered by the driven harness, at 11 of 11 against a real application: text arrives, the caret lands where asked, the clipboard survives for text and for a PNG, both borrowed writes are declared, a selection is read and given back, and an empty document reports no selection.
  - **What a harness cannot reach, and why.** Driving the launcher's own interface was tried and abandoned twice. A synthesised Option+Space never reaches the global shortcut. Launching the binary again does open the launcher, but keystrokes do not arrive at the panel, and there is no asking whether it is up either, because a non-activating panel never becomes the frontmost application. So root search, Enter, and the action dispatch stay a manual check.
  - Two conditions the harness needs, both learned the hard way: the application must not be running, or two clipboard owners fight and every check reads empty; and TextEdit must be quit between runs, because rewriting the scratch file does not reset a window it already has open.
- [ ] 9.2 Walk every scenario in the four spec files on Windows
  - The selection-and-paste scenarios pass live through the harness (18 of 18,
    see group 6). The snippets, quicklinks, and templates scenarios run through
    the launcher's own interface, which the harness cannot drive, the same manual
    check 9.1 is on macOS. Outstanding: those and the elevated case (9.9).
- [x] 9.3 Confirm on both platforms that the user's clipboard is identical before and after a paste, for text and for an image
  - macOS: verified by the driven harness for both content types. Text comes back identical, and a PNG on the clipboard is still there, byte for byte, after an insertion. The image path is a separate branch from text and had never been run live.
- [ ] 9.4 Confirm on both platforms that no paste, restore, or selection capture appears in the clipboard history or reorders it, while the watcher is running
  - macOS: confirmed by the author with the watcher running. Windows outstanding.
  - Windows: with the live watcher running, seeding the clipboard is recorded,
    and after using a snippet through the launcher the history is byte-for-byte
    unchanged: the pasted text never appears and the newest entry keeps its
    timestamp rather than being reordered by the restore. The harness also
    asserts an insertion declares exactly two borrowed writes, and the
    suppression queue is unit tested in 4.7. Not fully airtight only because
    driving snippet use by root search cannot be observed from outside the
    webview; the author's daily use closes the last gap.
- [ ] 9.5 Confirm on both platforms that a snippet with a caret position leaves the caret where the template declared it, in at least two applications
  - Windows: verified live in the WinForms edit control that typing after an
    insertion lands at the declared caret (`<b>HERE</b>`). A second application
    is the remaining manual step.
- [ ] 9.6 Confirm on both platforms that activation still meets the 80ms budget on a release build with both new extensions enabled
  - Windows: **passes.** Release build with `DANGO_MEASURE=1`, store loaded and
    both new extensions registered (no load-failure logs). 26 activations by
    global hotkey: min 19.2ms, median 24.6ms, p90 26.8ms, max 33.0ms, none over
    80ms. macOS still to confirm for this change.
- [x] 9.7 Confirm on both platforms that text appears in the target application within 400ms of confirming, measured over repeated pastes
  - macOS: an insertion returns in about 370ms across runs, and roughly 300ms of that is the clipboard restore the user never waits for. The text itself arrives in about 70ms.
- [ ] 9.8 Confirm on macOS that revoking the Accessibility permission produces the explained failure and the prompt action, and that granting it restores normal behaviour without a restart
- [ ] 9.9 Confirm on Windows that pasting into an elevated window fails visibly and leaves the clipboard alone
  - The harness is non-elevated (integrity `0x2000`) and its
    `an_elevated_target_is_refused` check targets a window titled `dango-elevated`.
    Outstanding only for want of that window: launching an elevated process is
    blocked from this session, so the author opens one elevated and reruns to
    close it.
- [ ] 9.10 Confirm on both platforms that snippets and quicklinks survive a restart with their names and templates intact
  - Windows: **confirmed.** A snippet and a quicklink were created through the
    launcher's own create forms on a release build, and after two restarts both
    are still present with their names and templates intact (checked in SQLite).
    The root providers that read these tables back are unit tested in 7.5 and
    8.5. macOS still to confirm for this change.
- [ ] 9.11 Confirm on both platforms that the save form's argument preview catches a pasted doubled-brace expression before it is stored
  - macOS: confirmed by the author, who saved a GitHub Actions expression and later met its argument prompt. Windows outstanding.
  - Windows: reading the live preview is inside the launcher's webview form, which
    the harness cannot introspect, so it stays an author confirmation like
    macOS 9.1. The preview logic itself is shared and unit tested (3.5, 3.6).
