## Context

See proposal.md - Why.

What `add-text-plumbing` left behind, which this is built on:

- `text::TextExchange::insert(text, caret)` puts text into the frontmost
  application and gives the clipboard back. Verified live on macOS against a
  real application: the text arrives, the clipboard survives for text and for
  images, and both borrowed writes are declared to the clipboard history.
- `text::Keys` already sends keystrokes through `enigo`, including
  `caret_left`. Backspaces are the same shape of operation.
- `text::Launcher::dismiss` exists because a keystroke goes wherever the focus
  is. Expansion has no launcher to dismiss, which removes a whole class of
  ordering bug that cost three attempts to find last time.
- The clipboard history has an exclusion list of application names and a
  `PreferencePolicy` that reads it per event, so editing it takes effect without
  a restart.
- macOS Accessibility is a state the user is shown rather than a silent failure.

Two facts about the platforms that shape everything below:

- macOS delivers keyboard events to an event tap, and the tap is disabled by the
  system if its callback is slow. Windows delivers them to a low-level hook on
  the thread that installed it, and blocks the input path while the callback
  runs.
- Both can say which characters an event produced, rather than only which key
  was pressed. That distinction turns out to decide the whole dependency
  question.

## Goals / Non-Goals

**Goals:**

- Typing a keyword produces the snippet's text, in any application, with nothing
  on screen.
- A component that watches every keystroke must be boring to reason about:
  small, bounded, forgetful, and obviously incapable of keeping anything.
- Never delay the machine. The monitor is on the system's input path and a slow
  callback there is felt everywhere, not just in Dango.
- Off means off. Disabling the extension removes the tap, not just the matching.

**Non-Goals:**

- See proposal.md - Non-goals. At design level: no attempt to know what the
  target application did with the keystrokes, and no attempt to verify that the
  keyword was actually deleted.

## Decisions

### Both platforms are hand-rolled, and it costs no new dependency

Evaluated: `keytap` 0.4.0, `rdev` 0.5.3, `inputbot` 0.6.0, `mki` 0.2.3,
`hookmap-core` 0.2.1, and the platform APIs directly.

The deciding question is not "can this crate observe keystrokes". Several can.
It is "can it say which *character* was produced", because matching typed text
against a keyword is a question about characters, and everything else is a
question about key positions.

`keytap` 0.4.0 was the strongest candidate and the closest call in this project
so far. It is recent, it is `rust-version = "1.85"`, it has observe-only
semantics, clean shutdown, and an explicit `KeyRepeat` event so consumers do not
have to de-duplicate. Best of all it depends on `objc2`, `objc2-core-foundation`
and `objc2-core-graphics`, which is this project's exact stack, so it would have
added no parallel FFI layer.

It reports `Key::A`, `Key::B`, and so on, and its own source calls them
"Letters (positional, QWERTY)". A positional key is not a character. On the
author's German layout the positions do not agree with the letters, dead keys
produce nothing until the next keystroke, and any layout that is not Latin
produces nothing this enum can express. Recovering characters would mean calling
`UCKeyTranslate` and `ToUnicodeEx` ourselves, which is the only genuinely hard
part of the job. A crate that leaves the hard part is not carrying its weight.

Rejected: `rdev` 0.5.3, same positional-key problem and a release that predates
the current `windows` and `objc2` generations. Rejected: `inputbot`, `mki`, and
`hookmap-core`, all of which are Windows and Linux only, so macOS would be
hand-rolled anyway and the crate would solve the half that is already easier.

What is left is not much, because the platforms hand the answer over:

- macOS: `CGEvent::tap_create` and `CGEvent::keyboard_get_unicode_string`, both
  in `objc2-core-graphics` 0.3, already a dependency of this project.
- Windows: `SetWindowsHookExW` with `WH_KEYBOARD_LL`, and `ToUnicodeEx` with
  `GetKeyboardState` and `GetKeyboardLayout`, all in `windows-sys` 0.61, already
  a dependency.

So this is the rare case where the off-the-shelf rule points at hand-rolling:
the maintained options do less than the platform does, and taking one would add
a dependency *and* leave the hard part. That conclusion is only allowed because
the candidates were checked rather than guessed at, which is what the rule
actually asks for.

### The keyword is a field on the file-backed snippet record

Since this was first planned, `add-file-backed-records` moved snippets out of
SQLite into `snippets.json`, whose records are held as JSON objects that preserve
unknown fields. So the keyword is a `keyword` field on the record, not a database
column, and it travels in the dotfiles with the snippet. Uniqueness is enforced
in the store at save time, and a snippet whose template has placeholder arguments
is refused a keyword there, with a message saying why. This replaces the original
migration-and-unique-index plan, which no longer has a table to attach to.

### The Windows monitor reuses the hook and injection built for the hyperkey

`add-hyperkey` built the Windows pieces this needs: a `WH_KEYBOARD_LL` hook on a
dedicated pumped thread, `SendInput` for synthetic keys, and the `DANGO_INJECTED`
`dwExtraInfo` marker (now shared in `platform::windows`) that lets a hook tell
Dango's own output from the user's. The monitor is a second consumer of that
pattern: it installs its own listening hook, ignores any event carrying
`DANGO_INJECTED` so its backspaces and inserts are never observed, and reads the
character a key produced with `ToUnicodeEx` over `GetKeyboardState` and
`GetKeyboardLayout`. It lives behind a `KeyMonitor` platform trait beside
`Hyperkey`, and Windows is built and verified first for the same reason.

Unlike the hyperkey hook, this one never swallows or rewrites a key: it returns
every event to the chain and only observes.

### The monitor keeps characters, never keys, and never more than it needs

The buffer is a fixed-size ring of the last N characters typed, where N is the
longest keyword plus one. Nothing longer is retained, because nothing longer can
match.

That bound is the privacy argument, and it is structural rather than a promise:
a component that physically cannot hold more than about thirty characters cannot
accumulate a password, a message, or a search query. It is the same shape as the
clipboard history's rule that exclusions are checked before content is read.

The buffer is cleared, not just aged out, on every one of:

- any key that is not a printable character, which includes Enter, Tab, Escape,
  the arrow keys, and every chord with a modifier other than Shift;
- the frontmost application changing;
- a pause longer than a few seconds;
- secure input becoming active;
- the extension being disabled.

Rejected: keeping a longer buffer to allow phrases with spaces. A keyword with a
space in it is worth less than a bound small enough to defend.

### Nothing is observed while the system says a password is being typed

macOS has `IsSecureEventInputEnabled()`. When a password field has secure input
active, the tap receives nothing useful anyway, but the check is made explicitly
and the buffer is cleared, so that the state is deliberate rather than an
accident of what the tap happened to deliver.

Windows has no equivalent: a low-level hook sees password fields. So the check
there is the one Windows can answer, `GetGUIThreadInfo` on the foreground thread
for a focused control, and the honest position is recorded in the risks: on
Windows the monitor can see what is typed into a password field, and the defence
is the bound on the buffer plus the exclusion list, not a platform guarantee.

That asymmetry is stated in the spec with a scenario per platform, because it is
a real difference in what the user is promised.

Rejected: trying to detect password fields by inspecting the accessibility tree
on every keystroke. It is slow, it is on the input path, and it is wrong often
enough to be worse than the honest bound.

### The exclusion list is shared with the clipboard history

An application the user does not want their clipboard recorded from is an
application they do not want their keystrokes observed in. Rather than a second
list to maintain, the monitor reads the one that exists, and expansion is
skipped there entirely.

This does mean one list governs two features, which is a coupling worth naming.
It is the right default and the wrong thing to make unchangeable, so the reading
happens through the same per-event preference read the watcher uses.

### The callback hands off immediately and never does work

On Windows the hook blocks the input path. On macOS a slow tap callback gets the
tap disabled by the system, which is a failure mode that looks exactly like the
feature quietly not working.

So the callback does one thing: translate the event to characters and push them
onto a channel. Matching, database reads, and every keystroke Dango sends back
happen on another thread. Nothing that can block, allocate unboundedly, or touch
SQLite runs inside the callback.

macOS additionally has to handle `kCGEventTapDisabledByTimeout` and
`kCGEventTapDisabledByUserInput` by re-enabling the tap, because the system
disables it rather than telling anyone. A tap that silently stops is the most
likely way this feature breaks in the field, so re-enabling is part of the
design rather than an afterthought.

### Expansion fires on a word boundary, not on any occurrence

A keyword matches when the characters before the caret are the keyword *and*
what preceded them was not a word character. So `sig` expands at the start of a
line or after a space, and does not expand inside `design`.

Rejected: expanding on any occurrence. The author would have to choose keywords
that never appear inside another word, which is a rule the feature imposes on
the user rather than absorbs.

Rejected: requiring a terminator key such as space or tab. It is more
predictable, and it is how several tools do it, but it means the expanded text
is always followed by the terminator or the terminator has to be swallowed, and
swallowing a keystroke is exactly the interception this design refuses to do.

The guidance that follows from this, rather than a rule the code enforces, is
that a distinctive keyword such as `;sig` is a better keyword than `sig`. A
leading punctuation character is always a word boundary, so it always matches.

### Deleting the keyword is backspaces, and nothing verifies it

Once matched, Dango sends one backspace per character of the keyword and then
inserts the text through the existing path.

There is no way to confirm the deletion worked. An application that autocorrects
or autocompletes while the keyword is typed may hold something other than what
was observed, and the backspaces will then delete the wrong thing. This is the
sharpest edge in the change, it cannot be closed without reading the target's
text on every keystroke, and it is stated in the risks rather than papered over.

The mitigation is ordering: backspaces first, then the insert, so the worst case
is text the user can see and fix, never a silent partial expansion.

### A snippet with placeholders cannot have a keyword

Expanding in place means no window. A template with arguments needs a window.
Rather than expand it with the arguments blank, which would be a silent wrong
answer, a snippet with arguments is refused a keyword at save time with a
message saying why. Reserved placeholders that resolve themselves, such as
`date` and `clipboard`, are fine and expand normally.

### The service follows the extension, and the tap follows the service

The monitor is a `Service` on the snippets extension, so it starts when the
extension is enabled and stops when it is disabled, which is the contract
`extension-model` already defines. Stopping removes the tap rather than leaving
it installed and ignoring events, because a key monitor that is installed but
idle is still a key monitor.

## Risks / Trade-offs

- **The keyword may not be what the application actually holds.** Autocorrect,
  autocomplete, and input method editors can change text between the keystroke
  and the field. → Backspaces run before the insert, so the failure is visible
  text rather than a silent corruption. No way to close it without reading the
  target on every keystroke.
- **On Windows the monitor can observe a password field.** There is no reliable
  equivalent of secure input. → The bound on the buffer and the exclusion list
  are the defence, and the spec says so per platform rather than implying parity.
- **A slow callback degrades the whole machine.** → The callback translates and
  hands off, nothing else. The verification measures typing latency with the
  monitor running, not just Dango's own responsiveness.
- **The macOS tap can be disabled by the system without notice.** → Handled
  explicitly by re-enabling on the disabled events. This is the most likely
  field failure and the easiest to miss, because everything keeps working except
  the feature.
- **One exclusion list governs two features.** → Correct default, stated
  coupling, read per event so it stays changeable.
- **A keyword that is a common word will expand when not wanted.** → The word
  boundary rule removes the worst of it; the rest is guidance about choosing
  distinctive keywords, not a rule the code can enforce without guessing.
- **This is the most invasive component in the project.** A bug here is felt in
  every application at once. → It is small, it is off unless the extension is
  enabled, and it is built on a path that is already verified, so the new
  surface is the monitor and nothing else.

## Open Questions

- Whether a per-snippet setting to require a terminator would be worth having
  for keywords the author wants to be conservative about. Deferred: it is a
  column and a branch, it changes nothing structural, and it is easier to judge
  after living with the word-boundary rule.
- Whether expansion should be suppressed in the launcher's own window. Almost
  certainly yes, but it may fall out for free depending on how the frontmost
  application is read, so it is a verification question rather than a design one.
