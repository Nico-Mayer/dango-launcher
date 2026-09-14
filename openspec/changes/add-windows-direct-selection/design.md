## Context

See `proposal.md` for motivation. What exists:

- `crate::text::DirectSelection` is a one-method trait: `selected_text() ->
  Option<String>`. macOS implements it as `MacSelection` over `AXSelectedText`;
  Windows passes `None` to `TextExchange::new`, so every Windows read goes
  through `selection_through_clipboard`.
- That fallback writes a sentinel to the clipboard, synthesizes Ctrl+C, waits
  for the clipboard to stop being the sentinel, and restores. The sentinel is
  what tells "nothing was selected" from "the selection happened to be what was
  already on the clipboard".
- The clipboard history already has an `excluded-applications` preference and a
  matcher for it, and keyword expansion reuses that same list.

## Goals / Non-Goals

**Goals:**

- Reading a selection on Windows sends no keystrokes wherever the application
  allows it.
- One order on both platforms: ask directly, fall back only when that fails.
- An application that cannot be asked and must not be typed at fails honestly
  rather than destructively.

**Non-Goals:**

- No change to insertion, to the clipboard restore, or to the macOS path.
- No general accessibility abstraction. One method, the one the trait already
  has.

## Decisions

### Use the `uiautomation` crate rather than driving COM directly

UI Automation is a COM API. Reaching it from Rust means either a binding crate
or hand-written COM through `windows`, which the project already depends on.

`uiautomation` (0.25.1, September 2026, ~285k recent downloads) wraps the
interfaces this needs, including `UITextPattern::get_selection()` returning text
ranges, and handles the COM initialisation and lifetime rules that are the
actual work. It is the off-the-shelf answer.

Alternatives considered:

- **Raw COM via the `windows` crate.** Already a dependency, so no new one. But
  it means `CoInitializeEx` handling, `IUIAutomation` creation, interface
  casting, `BSTR` lifetimes, and release discipline written by hand, for a
  standard job. Rejected under the off-the-shelf rule, which exists for exactly
  this.
- **`accesskit`.** Exposes an application's own tree to assistive technology.
  The wrong direction: Dango needs to read another process's tree.
- **`windows-accessibility` / thin `IAccessible` wrappers.** Legacy MSAA gets a
  selection from some Win32 edit controls but not from modern editors, which
  expose UIA instead. Worth reaching for only as a second attempt, and not in
  this change.

### How the selection is found

`GetFocusedElement`, then its `TextPattern`, then `get_selection()`, then the
text of the first range. That is the sequence the same API serves screen readers
with, so anything that works with a screen reader works here.

Two limits, both accepted:

- An application that exposes no text pattern answers nothing, and the fallback
  runs. That is expected for custom-rendered editors, which is the case that
  started this change.
- A multi-range selection (column selection) returns the first range only.
  Everything downstream takes one string.

### Answering, but empty, is not a failure

The current fallback cannot tell "no selection" from "cannot ask"; the sentinel
exists because of that. UI Automation can: an element that hands back a text
pattern and an empty selection has answered. Treating that as an empty selection
rather than a reason to press Ctrl+C is most of the value here, because it is
what stops the chord being sent to an editor that simply had nothing selected.

So `DirectSelection` needs a three-way answer, not `Option<String>`:

```rust
pub enum Selected {
    Text(String),
    Empty,
    Unavailable,
}
```

`MacSelection` maps its current `None` to `Unavailable` to keep today's
behaviour, since `AXSelectedText` does not distinguish the two either. Changing
the trait touches macOS, which is why this change verifies macOS as unchanged
rather than assuming it.

### A hung application must not hold the launcher

A cross-process COM call into a busy application can block, and the spec gives
the whole read 300ms. The call runs on a worker thread with the result on a
channel, and the caller waits 200ms before giving up, leaving 100ms for the
fallback to at least start.

A COM call that never returns leaks that thread. Accepted: it is one thread, it
ends when the target finally answers, and the alternative is blocking the
launcher.

### The exclusion list

A new `selection` block beside `hyperkey`, both of which belong to the
application rather than to any extension:

```jsonc
"selection": {
  // Never press the copy chord in these, whatever they are called on this
  // machine. Matched the way the clipboard history matches its own exclusions.
  "excluded-applications": "Zed"
}
```

It gates only the fallback. A named application that does expose its selection
is read directly, so the list costs nothing where it is not needed. Empty by
default: an exclusion that ships by default would silently disable a working
feature for someone whose setup does not have the problem.

### Interface copy

One new string, written against `openspec/specs/interface-copy/spec.md`:

- Fallback refused for a named application: "Dango can't read the selection in
  {application}. Copy the text first, then run this command."

`{application}` is the name the platform reports, the same name the user wrote
in their exclusion list. Existing messages cover every other path.

No new interface surface, so no primitive is involved.

## Risks / Trade-offs

- **Zed may expose no text pattern.** → Then this change stops the damage
  without restoring the feature there, and the exclusion list is what the user
  gets. The first task is a spike that answers it before anything is built, so
  the answer is known rather than assumed.
- **UI Automation is slower than an in-process call.** → Budgeted at 200ms with a
  fallback behind it, and measured against the spec's 300ms in verification.
- **The trait signature changes, so macOS code changes.** → Mechanical, and the
  macOS behaviour is pinned by its existing scenarios, which must still pass.
- **A new dependency for one call.** → Justified by the COM lifetime work it
  replaces, and confined behind `DirectSelection`; the crate is not referenced
  anywhere else.
- **The exclusion list is a manual workaround.** → It is, and it is the only
  honest one for an application that exposes nothing and binds the chord. It is
  documented as a guard, not as the feature.
