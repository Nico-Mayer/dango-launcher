## Context

See proposal.md - Why.

What already exists and shapes this:

- `platform::LauncherWindow::restore_previous_focus` is implemented on both
  platforms, but the two are not symmetric. Windows records the previous
  foreground window and gives it back with `AllowSetForegroundWindow` plus a
  forced foreground. macOS is a deliberate no-op, because the launcher is a
  non-activating `NSPanel`: it takes key window status without activating the
  application, so the application the user was in never stopped being the active
  one. Paste has to be correct under both of those, not under an average of them.
- `CrateClipboard` reads and writes text and images through `clipboard-rs`, and
  the watcher suppresses Dango's own write. The suppression is one shot and
  content-matched: `was_ours` takes the stored `Content` and compares it, so it
  covers exactly one change. A paste makes two writes.
- `protocol::FormView` and `FormField` exist, `ProtocolView.svelte` renders a
  form and calls `onsubmit(formValues)`, and `App.svelte` passes
  `onsubmit={() => {}}`. There is no Tauri command that accepts field values.
  The path is built up to the last inch and then stops.
- `Extension::perform_action(item_id, action_id) -> ActionOutcome` is how a
  result is acted on, and `ActionOutcome::Replaced` already lets an action leave
  the user where they were.
- `SearchPipeline` holds `providers: Vec<Arc<dyn RootProvider>>`, so two more
  extensions contributing root items needs no pipeline change.
- The store has a syncable convention (UUID `id`, `updated_at`, `deleted_at`)
  and a `local_` convention. Clipboard history used `local_`; snippets and
  quicklinks are the first content that should follow the syncable one.

## Goals / Non-Goals

**Goals:**

- One path into another application, used by everything that ever needs one, so
  M4, M5, and M6 find it finished rather than each building a variant.
- The user's clipboard is theirs. Borrowing it to paste is an implementation
  detail they must never be able to observe, in the clipboard's contents or in
  the history.
- A placeholder vocabulary small enough to hold in the head and identical
  wherever it appears.
- Failing loudly when the permission is missing. A paste that silently does
  nothing is the worst outcome available.

**Non-Goals:**

- See proposal.md - Non-goals. At design level, additionally: no attempt to
  detect that the target application has actually consumed the paste on macOS,
  because no such signal exists; and no second input mechanism for arguments
  beyond the form the view protocol already defines.

## Decisions

### `enigo` synthesises the keystrokes on both platforms

Evaluated: `enigo` 0.6.1, `rdev` 0.5.3, `winput` 0.2.5, and hand-rolling
`SendInput` and `CGEventPost`.

`enigo` is the one, and its source was read rather than guessed at:

- macOS posts through `CGEventPost(CGEventTapLocation::HID)`, and it already
  carries the permission question: it declares `AXIsProcessTrustedWithOptions`
  and can prompt, behind the `open_prompt_to_get_permissions` setting.
- Windows posts through `SendInput` with `KEYEVENTF_SCANCODE` and
  `MapVirtualKeyExW` against the foreground layout, which is the part worth not
  owning. A paste shortcut is a physical key position, and getting that wrong
  means Dango works on a US layout and pastes garbage on a German one.
- `Settings::independent_of_keyboard_state` (macOS, default true) makes the
  injected event ignore keys the user is physically holding. That matters here
  more than it looks: the user reaches this path by pressing a hotkey, and their
  modifiers may still be down when the paste goes out.
- `Settings::windows_dw_extra_info` tags every event Dango injects, which is what
  M5's key monitor will need to avoid hearing its own output.
- Its Linux dependencies (`x11rb`, `wayland-client`, `xkbcommon`) are gated
  behind `cfg(all(unix, not(target_os = "macos")))`, so nothing Linux enters
  this build. `default-features = false` keeps it that way.

Rejected: `rdev` 0.5.3. It sends and listens, but it is listener-first, its
API has no way to mark an injected event as its own, and it carries none of the
macOS permission handling. Its last release predates the current `windows` and
`objc2` generations.

Rejected: `winput` 0.2.5. A good Windows crate, but Windows only, and built on
`winapi`, which this project does not use anywhere. Adopting it would leave
macOS hand-rolled and add a third Windows FFI stack next to `windows` and
`windows-sys`. A crate that solves one of two platforms does not solve the
problem this project has.

Rejected: hand-rolled. The project already has
`Win32_UI_Input_KeyboardAndMouse` and could call `SendInput` directly, and that
is exactly the trap: the call is easy and everything around it is not. Layout
mapping, scan codes, dead keys, extended-key flags, and the macOS trust prompt
are all work someone has already done correctly twice.

### `objc2-application-services` reads the macOS selection

Evaluated: `objc2-application-services` 0.3.2, `axuielement` 0.9.1,
`accessibility` 0.2.0, and `macos-accessibility-client` 0.0.2.

`objc2-application-services` wins on one argument that outweighs ergonomics: it
is the same `objc2` family this project already depends on. Its
`HIServices/AXUIElement` module declares `AXUIElementCopyAttributeValue`,
`AXUIElementCreateSystemWide`, `AXIsProcessTrusted`, and
`AXIsProcessTrustedWithOptions`, and they take the `CFDictionary` and `CFString`
types from `objc2-core-foundation` 0.3, which is already in the tree via
`objc2-app-kit`. Nothing converts at a boundary because there is no boundary.

It does not export the attribute name constants, so `AXFocusedUIElement` and
`AXSelectedText` are written as string literals. They are documented and stable,
and the spike confirmed both against a live element.

Rejected: `axuielement` 0.9.1. The nicest API of the four, with safe wrappers
and the `AXSelectedText` constants named, and it was the first choice until its
manifest was read: it depends on `apple-cf`, a separate Core Foundation binding
stack. Taking it means two incompatible `CFString` types in one binary and
conversions wherever AX meets AppKit. That is a real cost paid forever for
convenience in one module.

An honesty note, since the spike made it visible: `enigo` pulls a second Core
Foundation stack anyway, `core-foundation` 0.10 and `core-graphics` 0.25. The
distinction that matters is narrower than "one stack in the binary". Those types
never surface in our code, because `enigo`'s API is its own `Key` enum, whereas
AX values would have crossed into our own AppKit code on every selection read.
The reasoning holds; it was stated too broadly.

Rejected: `accessibility` 0.2.0. Depends on `cocoa` 0.26, the pre-`objc2`
generation the whole ecosystem has moved off. Same two-stack problem, older.

Rejected: `macos-accessibility-client` 0.0.2. It wraps the trust check and
nothing else, which `objc2-application-services` already provides. A dependency
for one function call that is already in the tree.

### Reading the selection is AX first, clipboard round trip as the shared fallback

macOS asks the accessibility API for the focused element and then its
`AXSelectedText`. Nothing is copied, the clipboard is untouched, and it is fast.

Which element to start from was settled by measurement rather than by the usual
example. The system-wide element, which every tutorial reaches for first,
answered in none of the applications tried and returned `CannotComplete` every
time. The application's own element answers for native applications and for a
terminal. So the application element is asked first, and the system-wide one is
kept only as a fallback because failing costs it around 15µs, against roughly
35ms for the application element when it declines.

It does not always answer. An application that has not turned its accessibility
tree on, or a control that implements no text attribute, returns nothing, and
"nothing" is indistinguishable from "no selection".

Windows has no equivalent worth having, which the project context already
records, so its route is: copy, read the clipboard, put the clipboard back.

Rather than two unrelated implementations, the clipboard round trip is written
once as the platform-neutral fallback, on the clipboard access and the key
injection this change already has, and macOS layers AX on top of it. So there is
one tricky piece of code, exercised on both platforms, rather than a Windows one
that is only ever tested on the machine that cannot be compiled locally.

Rejected: `uiautomation` 0.25.1 for the Windows selection. `TextPattern` and
`GetSelection` are the right shape, but they only answer in applications that
implement the UIA text pattern. Terminals, Electron applications, and plenty of
Win32 controls do not, so it would be a path that works sometimes and falls back
often, and the fallback is the thing that has to be right anyway. A second
mechanism that does not remove the first is not worth its dependency.

### A sentinel tells "nothing selected" from "selected what was already there"

Found while building the shared round trip, and it was not in the original
design.

Pressing copy with nothing selected leaves the clipboard exactly as it was. So
reading the clipboard afterwards cannot tell an empty selection from a selection
that happens to match what the user already had on the clipboard, and the
difference matters: one should report nothing, the other should report the text.

So the round trip writes a sentinel before the copy. If the sentinel survives,
the copy did nothing and there was no selection. It costs one extra clipboard
write, which is declared to the history like every other, and the suppression
queue is sized for it.

The unavoidable case is stated rather than solved: if the user copies at the
exact moment the copy keystroke lands, what comes back is theirs and nothing can
tell. That window is a few milliseconds wide and there is no signal that would
close it.

### The restore compares against what Dango borrowed, from one read

The restore is abandoned when the clipboard no longer holds what Dango put
there, because that means the user has copied something of their own and
overwriting it is the one failure they would actually notice.

The subtlety is which value to compare against. Re-reading the clipboard just
before restoring defeats the whole check, since it would read the user's new
content and conclude that it was the thing being borrowed. So the comparison
uses the value read once, immediately after the copy or the paste, and anything
appearing after that point is the user's.

### Pasting goes through the clipboard, not through typing the text

`enigo` can type a string character by character, and that needs no clipboard at
all. It loses anyway: it is slow for anything longer than a line, it goes wrong
with input method editors and with applications that autocomplete as you type,
and the failure mode is a half-entered snippet.

So: save the clipboard, write the text, send the paste shortcut, restore the
clipboard. The cost is that the clipboard is borrowed for a moment, and the two
decisions below are about paying it honestly.

### The launcher gets out of the way first, and each platform is asked differently

The keystroke goes wherever the input focus is, so the ordering is the whole
correctness argument.

- Windows: hide the launcher, call `restore_previous_focus`, then wait until
  `GetForegroundWindow` actually returns the target window before injecting.
  Foreground lock means the restore can be refused, and a fixed delay would
  paste into whatever won instead. Asking the system who is in front is a real
  answer; sleeping is a guess.
- macOS: hide the panel and let key status fall back. The application was never
  deactivated, so there is nothing to restore, and `restore_previous_focus`
  stays the no-op it is. What is needed is that the panel has actually resigned
  key before the event is posted, which is a settle wait rather than a race
  against another process.

This is why the two platforms get separate code rather than a shared "restore
focus and paste" helper. They are not the same operation wearing different
names.

### Windows waits for the target to finish pasting, the way the watcher waits for the copier

Restoring the clipboard too early means restoring it before the target has read
it, and the user pastes the old contents. There is no "the paste is done" event
on either platform.

Windows has half an answer, and it is the same one the clipboard watcher already
uses in the other direction: ask the target window to answer a no-op message and
wait for the reply. A window that answers is back in its message loop, which
means it has finished handling the paste it was sent. The archived change found
this the hard way with .NET and `CLIPBRD_E_CANT_OPEN`; this is the same trick
pointed the other way, and it is worth reusing rather than rediscovering.

macOS has no equivalent, so the restore happens after a delay the spike
measured: 80ms loses the race intermittently in Notes, 100ms upward never did,
and the constant carries headroom over the failure rather than sitting on the
first success. That is still a race, it is stated in the risks, and it is the
same race every launcher on the platform runs.

Reading a selection needs none of this, which the measurement made obvious. A
copy leaves something observable, because the sentinel stops being on the
clipboard, so the exchange watches for that and returns the moment it happens
rather than waiting a fixed time. Only paste waits blindly, because only paste
has nothing to watch. Sharing one delay between them would have put a
paste-sized wait inside the 300ms a selection read is given.

### Suppression becomes a small queue, not a single slot

A paste writes the clipboard twice: the text going out, and the user's content
going back. The watcher's `ours` field holds one `Content` and `was_ours` takes
it, so the second write would be recorded: the pasted text would land in the
history, and restoring the user's previous content would reorder its entry to
newest.

So suppression holds a short list of pending writes rather than one, and a
change matching any of them is consumed and dropped. It stays content-matched
and one-shot per entry, so the property the archived design wanted still holds:
the user genuinely re-copying the same thing from another application is still
recorded.

Rejected: suppressing by time window. A window that is long enough to cover the
paste and the restore is long enough to swallow something the user copied in
between, and this feature's whole reputation is not losing things.

### `minijinja` renders, and an unknown variable is an argument

Evaluated: `minijinja` 2.24.0, `upon` 0.11.0, `handlebars` 6.4.4, `tera`,
`liquid`, `tinytemplate` 1.2.1, and `strfmt` 0.2.5.

The deciding question was not rendering. Every one of them renders. It was: what
does this template need before it can be rendered? A snippet with a placeholder
the user has to fill in must produce a form, and producing a form means knowing
the names, before rendering, without running anything.

`minijinja` answers it directly. `Template::undeclared_variables(nested)`
re-parses the template and returns the names it references. That gives the whole
design its shape:

- A fixed reserved vocabulary resolves itself: `clipboard`, `selection`,
  `date`, `uuid`, `cursor`, `query`.
- Every other name the template references is an argument, labelled by its own
  name, and the user is asked for it.

So there is no `{{ argument(name="City") }}` syntax to learn. `{{ city }}` is an
argument because nothing else claims the name. One rule, and the engine already
computes the set.

The features taken are narrow: `default-features = false`, plus `builtins`,
`serde`, and `urlencode`. Quicklinks need a query put into a URL safely and
`minijinja::filters::urlencode` is that, behind its own feature flag. `serde` is
not optional in practice: without it `Environment::new` is deprecated, and CI
turns that warning into a failure. No loader is installed, so `include` and
`extends` have nothing to reach.

Rejected: `upon` 0.11.0. Genuinely close, and the runner-up: minimal
dependencies, configurable delimiters, filters, a clean API. It has no way to
report which variables a template references. `render_from_fn` takes a lookup
closure and could collect names during a throwaway render, but a render only
visits the branches it takes, and a render is not a parse. Recovering the
argument list would mean parsing the template ourselves next to an engine that
already parsed it, which is the hand-rolling this project's rules exist to
prevent.

Rejected: `handlebars` 6.4.4, `tera`, and `liquid`. Full engines with helper
registries, partials, and inheritance. More surface than a snippet needs and the
same enumeration gap.

Rejected: `tinytemplate` 1.2.1. Last released in 2021.

Rejected: `strfmt` 0.2.5. Substitution into `{name}` and nothing more, so date
formatting and URL encoding would be hand-rolled around it. It is the closest
thing to writing it ourselves while still taking a dependency.

### Delimiters stay `{{ }}`, and the save form says what a template will ask for

Revised after the spike, which half disproved the original argument.

`minijinja` can change its delimiters behind the `custom_syntax` feature, and
single braces were tempting because `{clipboard}` reads better than
`{{ clipboard }}`.

Single braces stay rejected, and the spike confirmed the reason: a snippet of
code full of `{` reports no placeholders under `{{ }}`, so the common case is
already free. Making `{` significant would tax it to save two characters.

What the original argument missed is that doubled braces are not rare in the
text an author stores as a snippet, and they fail in two different ways:

- `printf("{{%d}}", x)` does not parse, so the snippet is refused at save.
  Annoying, but visible.
- `runs-on: ${{ matrix.os }}` parses and reports an argument named `matrix`.
  `<p>{{ user.name }}</p>` reports `user`. These save cleanly and then ask the
  wrong question every single time they are used. GitHub Actions, Vue, Angular,
  and Handlebars all land here.

The silent one is the problem, so the fix is to remove the silence rather than
the syntax: the create and edit forms show the arguments the template will ask
for, live, as the template is typed. Pasting a workflow file and seeing "this
will ask you for: matrix" puts the surprise at the one moment the user is
already editing the thing and can fix it.

`minijinja`'s escapes both work and were confirmed: `{% raw %}...{% endraw %}`
and `{{ '{{' }}` each parse to no placeholders. So there is a fix available once
the user knows there is something to fix, which is exactly what the preview
gives them.

Rejected: switching to `<<name>>` or `[[name]]` under `custom_syntax`. It costs
no new dependency, since `aho-corasick` is already in the tree, but every
delimiter pair collides with something. `[[` is bash test syntax and Lua long
strings; `<<` is heredocs and Erlang binaries. Moving the collision is not
removing it, and `{{ }}` is at least a convention the author already reads
elsewhere.

Rejected: rejecting any template whose arguments look like they came from code.
There is no honest rule for that, and a validator that guesses is worse than a
preview that shows.

The other consequence of `{{ }}` is that `{% if %}` and `{% for %}` are
reachable, because block syntax comes from the parser and no feature flag
removes it. They are not specified, not documented, and not tested, and they are
not blocked either: rejecting a template the engine would render happily is a
validator we would own and a rule we would have to explain. The specs cover
placeholders. Anything else a template does is between the author and the
engine.

### Arguments are a form, and a form submit is an action carrying values

Confirming a snippet that needs arguments pushes a `FormView` with one field per
argument. Submitting renders the template and pastes.

That needs the form submit path, which `view-protocol` has required since M1 and
nothing implements. The shape chosen is the smallest one that fits what is
already there: a form submit is an action on the form, so the action path gains
the field values.

`Extension::perform_action` becomes
`perform_action(item_id, action_id, values: &HashMap<String, String>)`, the
`run_action` Tauri command gains the same parameter, and `ProtocolView`'s
`onsubmit` calls it with the form's primary action and the collected values.
`ActionOutcome::Replaced` already covers a submit that should leave the user in
place, and the three existing extensions ignore the new parameter.

Rejected: a separate `submit_form` command and a separate trait method. It
doubles the dispatch path for something the action path already models, and it
would leave two ways to send a form's contents back depending on whether the
user pressed Enter or clicked an action.

Field order is by first occurrence in the template source, because
`undeclared_variables` returns a `HashSet` and a form whose fields shuffle
between openings is unusable. That is a scan of the source for ordering only;
the set of names still comes from the engine.

### The argument preview is a field kind, not a live form round trip

The preview decided above has to update as the user types, and the view protocol
has no way for a form to tell a command that a field changed.

Three routes were considered.

Rejected: a per-field "notify on change" flag, so the command is told and pushes
a replaced view. It is the most consistent with the architecture, since
`Filtering::Command` already means exactly this for lists and the protocol is
full-tree replace anyway. It loses on one detail: `ProtocolView` keeps the
entered values in local state, so replacing the tree on every keystroke would
fight the user's own typing and caret. A round trip per keystroke to rebuild the
form the user is currently inside is the wrong shape.

Rejected: showing the arguments only after a save attempt. That is where the
surprise already is, and moving it nowhere is not a fix.

Chosen: `FieldKind` gains a `Template` variant next to `Text`, `Password`, and
`Toggle`. A field declared as a template renders with its argument preview
beneath it, fed by a Tauri command that parses the text and answers with the
arguments in order, or with the parse error. No view tree is replaced, the
user's typing is untouched, and any extension that stores a template gets the
same affordance for free.

This is additive to the view protocol: a new enum variant, no change to any
existing shape, and nothing that was valid before stops being valid.
`protocolVersion` does not move. It is recorded here because the protocol is a
versioned contract and even additive changes to it should be deliberate.

The command is the only new surface, and it is pure: text in, argument names and
a possible parse error out. Nothing is stored and nothing is rendered.

### `query` is a reserved argument name, not a second mechanism

A quicklink's query goes through the same form as any other argument. The name
is reserved only so that a quicklink can rely on it and so `{{ query|urlencode }}`
means the same thing everywhere.

This is why quicklinks are cheap once snippets exist: they are the same
template, the same form, and a different final action.

### The caret lands where `cursor` was, by counting back

`{{ cursor }}` renders to a sentinel from the Unicode private use area. After
rendering, the text is split at the sentinel, the sentinel is removed, the whole
text is pasted, and the caret is moved left once per character after the split
point.

It is not elegant and it is the only thing that works without the target
application's cooperation. A sentinel from the private use area rather than a
common character because it must not collide with content the user wrote, and a
count of characters rather than bytes because the arrow key moves by neither
bytes nor graphemes reliably. Only one `cursor` is honoured; a second renders to
nothing.

Rejected: setting the selection through the accessibility API after pasting.
macOS could, Windows generally could not, and the two would disagree.

### Snippets and quicklinks are user records, so they are rows, not manifest entries

A `CommandDecl` is static, declared in a manifest, and known at build time. A
snippet is created by the user at half past three on a Tuesday. So each
extension declares its static commands in its manifest ("Create Snippet",
"Search Snippets") and contributes the user's own records through a root items
provider, which is exactly the split `extension-model` already defines and what
`applications` does with the machine's applications.

Both tables follow the syncable convention rather than `local_`. Clipboard
history was machine-local because a shared clipboard history is a hazard; a
snippet the author writes on macOS and wants on Windows is the entire reason M9
exists.

### The permission is a state the user is shown, not an error they decode

Everything here fails on macOS without Accessibility, and the failure is silent
at the OS level: the event is posted and discarded.

So `AXIsProcessTrusted` is checked before acting, and a paste that cannot happen
says so with an action that opens the prompt.
`enigo`'s `open_prompt_to_get_permissions` is set to false, because a system
dialog appearing the first time a user pastes, from a process they cannot see,
is worse than a message in the launcher that explains itself and offers the
prompt deliberately.

Windows needs no permission and so has no such state, except for the ceiling the
project context already records: a non-elevated Dango cannot inject into an
elevated window. That fails visibly rather than mysteriously.

## Risks / Trade-offs

- **The macOS clipboard restore is a timed guess.** Too early and the target
  pastes the old contents; too late and the user's own next copy races it. →
  The delay is measured in the spike rather than picked, and the restore is
  skipped entirely if the clipboard changed underneath us, since that means the
  user has moved on and their content is no longer what we saved.
- **Accessibility trust is per binary and unsigned binaries change identity on
  every build.** Development may mean re-granting the permission after rebuilds,
  which is friction on the machine where the work happens. → Recorded so it is
  recognised as expected rather than debugged as a bug. The spike confirms how
  often it actually bites before anything is designed around it.
- **A paste into an application that ignores the shortcut does nothing
  visible.** Some applications remap paste; some fullscreen games swallow
  synthetic input entirely. → Out of reach. The keystroke is sent and there is
  no acknowledgement to wait for; this is the ceiling of the approach and every
  tool of this kind shares it.
- **The clipboard round trip for selection capture disturbs the clipboard on
  Windows always, and on macOS whenever AX declines to answer.** → The same
  suppression queue covers it, and the clipboard is restored the same way. The
  user-visible consequence is that a selection capture and a paste are the same
  cost, which is worth stating in the spec rather than hiding.
- **`undeclared_variables` returns an unordered set.** → Ordering is recovered
  by scanning the source, which is a small piece of code this change owns. It
  orders; it does not parse.
- **Two more root providers compete for the 50ms budget.** Both read small
  tables the user wrote by hand, so neither should be near it. → The spec carries
  the budget and verification measures it with a realistically sized set rather
  than three rows.
- **The action signature change touches every extension.** → Three call sites,
  all built-in, all ignoring the new parameter. Done now while there are three;
  it is the kind of change that becomes expensive exactly once third-party
  extensions exist, which is why it happens before M8 rather than during it.
- **Windows lands unverified.** Nothing here can be compiled on macOS, and the
  paste path is full of the foreground and message-loop behaviour that only
  shows at run time. → The same arrangement the clipboard change used: the code
  ships with the change, CI compiles it, and the confirmation tasks stay open
  until the author runs them on Windows.

## Open Questions

- Whether a snippet should be able to declare a keyword that expands it as the
  user types in any application, rather than being chosen from root search.
  Deferred: it needs a global key monitor, which is M5's, and it changes no spec
  here. The template engine and the paste path are the same either way.
- Whether a quicklink should be able to open in a chosen browser rather than the
  default one. Deferred: it is an extra column and an extra field, and nothing
  about the design resists it.
