## 1. Spike: reading the clipboard on both platforms

- [x] 1.1 On macOS, read the change counter, the text, and an image, and confirm the counter moves exactly once per copy
  - macOS: the counter does **not** move exactly once per copy. It advanced by two on ordinary copies, because clearing and writing each bump it. Text and images both read correctly; a PNG came back in 6.3ms.
- [x] 1.2 On Windows, read the sequence number, the text, and an image, and confirm it moves exactly once per copy
  - Windows: it does not move exactly once per copy either. One .NET copy moved it by 3 and then by 9 and delivered two or three change events, because the application announces its formats first and flushes them afterwards. Text and images both read correctly. Nothing reads the number any more: the crate delivers the events.
- [x] 1.3 On macOS, copy from a password manager and confirm the concealed marker is present before any content is read
  - macOS: **failed**. Proton Pass offers a copied password as `public.utf8-plain-text` and `NSStringPboardType` and nothing else. No concealed marker, no auto-generated marker, nothing to check.
- [x] 1.4 On Windows, copy from a password manager and confirm the exclusion formats are present before any content is read
  - Windows: **not confirmed against Proton Pass**, which was locked behind the account password on the test machine, and only its owner can unlock it. What was confirmed: `available_formats()` returns `ExcludeClipboardContentFromMonitorProcessing` and `CanIncludeInClipboardHistory` under exactly the names the watcher compares, and content carrying either is kept out without being read. Whether Proton Pass sets one on Windows is still open; it set nothing on macOS, and nothing depends on it.
- [x] 1.5 On Windows, identify the copying application through the clipboard owner, and on macOS through the frontmost application, and record how often macOS attributes wrongly
  - macOS: **failed**. A Proton Pass password was attributed to the terminal, because the copy was followed by switching away inside the poll interval. That is the normal password flow, not an edge case, so attribution at an instant is systematically wrong where it matters most.
  - Windows: the clipboard owner names the copying application exactly. PowerShell, Bruno (an Electron application, so the same clipboard path Proton Pass uses) and a window of our own all came back as themselves, and the Proton Pass process itself identifies as `Proton Pass`, the name on the default list. No wrong attribution was seen. Two cases leave no owner and so record: a program that opens the clipboard without a window (`clip.exe` does, and so does `clipboard-rs` itself), and a program that exits before the change is read.
- [x] 1.6 Measure what reading a large image costs, to size the poll interval against it
  - Reading the type list took 0.3 to 2.5ms warm and 9.5ms on the first call. Text read in under 1ms. A small PNG plus its TIFF took 6.3ms. A 250ms poll has ample room. TIFF was 37,482 bytes against 1,571 for the same image as PNG, so PNG is read first and TIFF only as a fallback.
- [x] 1.7 If any of the above does not work, revisit the design before building on it
  - Fired. Two design decisions were overturned and the design was revised before group 5: the marker is demoted from the defence to a bonus, attribution now covers the whole interval a change fell in rather than an instant, and the exclusion list ships with mainstream password managers already in it.

## 2. Preference storage

- [x] 2.1 Add typed get and set on the store, scoped by extension and optional command, tested against an in-memory store
- [x] 2.2 Return the declared default for a preference that was never set, with a test
- [x] 2.3 Return the declared default when a stored value no longer parses as its declared type, with a test that the extension still loads
- [x] 2.4 Keep extension-level and command-level values with the same key apart, with a test
- [x] 2.5 Hand an extension a reader bound to its own manifest, and test that it cannot read another extension's values
- [x] 2.6 Confirm a value survives a restart, with a test over a reopened store

## 3. History storage

- [x] 3.1 Add the migration for the history as a `local_` table, and confirm it applies to an existing database without touching the other tables
- [x] 3.2 Record a text entry and read the history back newest first, with a test
- [x] 3.3 Record an image entry as a file plus a row, and confirm the row points at a file that exists
- [x] 3.4 Move an existing entry to newest instead of inserting a duplicate, with a test over the same content copied twice
- [x] 3.5 Enforce the entry-count bound, discarding oldest first, with a test
- [x] 3.6 Enforce the total-size bound, discarding oldest first, with a test
- [x] 3.7 Refuse an entry over the per-entry ceiling without disturbing the history, with a test
- [x] 3.8 Delete an image's file when its entry is discarded or removed, with a test that the file is gone
- [x] 3.9 Remove an entry whose file has vanished and report the failure, with a test

## 4. The watcher

- [x] 4.1 Define the `ClipboardSource` trait covering the counter, the markers, the owning application, and reading text and images
- [x] 4.2 Implement the watcher service polling the counter, tested against a fake source driven by hand
- [x] 4.3 Check the markers and the exclusion list before reading any content, with a test that excluded content is never read
- [x] 4.4 Read the exclusion list per change so editing it takes effect without a restart, with a test
- [x] 4.5 Ignore the write Dango itself makes when restoring an entry, with a test
- [x] 4.6 Stop the watcher cleanly when the extension is disabled, with a test
- [x] 4.7 Confirm the watcher runs off the activation path, with a test that no clipboard work happens on the activation path
  - The service owns a thread and the activation path never reaches it. The live budget check is group 8.3.

## 5. Platform: macOS

- [x] 5.1 Read the change counter, verified against copying by hand
  - Superseded: there is no counter any more. `clipboard-rs` delivers a change event, so nothing polls. Confirmed live against a real clipboard.
- [x] 5.2 Detect the concealed and auto-generated markers, verified against a password manager
  - Now read from the crate's format list, which was confirmed to return the same types the hand-rolled code saw, including on a Proton Pass password. Nothing depends on them: Proton Pass sets neither.
- [x] 5.3 Read text and images, verified against copying each by hand
  - The crate's job now, on both platforms. Live tests round-trip text and an image, and recording both was confirmed against the running application.
  - One cost: the crate decodes and re-encodes, so the same PNG came back 5,427 bytes against 1,571 through the hand-rolled path. It spends the size budget faster.
- [x] 5.4 Name the frontmost application as the copying application, verified against copying from a known application
  - Not the frontmost application: activations are recorded as they happen and a change is attributed to everything frontmost around it.
  - Verified live against Proton Pass, which is what the first spike could not catch. Copying a password and switching straight back now yields candidates `["Proton Pass"]` and the content is never read. The same action under the old design attributed it to the terminal.
  - One finding on the way: workspace activation notifications only arrive while a run loop is pumping. The first probe blocked the main thread in a sleep loop and saw no activations at all, which looked like the fix failing. The real application runs AppKit's loop on the main thread and polls on another, which is what the probe now does too.

## 6. Platform: Windows

- [x] 6.1 Read the sequence number, verified against copying by hand
  - Superseded the same way as 5.1: the crate delivers a change event on `WM_CLIPBOARDUPDATE`. Confirmed live against real copies, including that one copy can arrive as two or three events.
- [x] 6.2 Detect the exclusion formats, verified against a password manager
  - Both formats are read from the crate's format list and keep content out unread, confirmed live with text carrying each one. Proton Pass itself was locked and could not be exercised; see 1.4.
- [x] 6.3 Read text and images, verified against copying each by hand
  - The crate's live round-trip tests pass on Windows. They failed at first for two reasons worth knowing: the text assertion looked for `text` and Windows names the format `CF_UNICODETEXT`, and the three live tests share one clipboard, so they only pass with `--test-threads=1`. The image round trip also failed once while the spike was reading the same clipboard: `clipboard-win` gives up after ten immediate retries to open it.
  - Recording both was confirmed against the running application. A .NET image came back as PNG through the crate; a 600x400 one cost 6,593 bytes.
- [x] 6.4 Name the clipboard owner as the copying application, verified against copying from a known application
  - `GetClipboardOwner`, then the owner's process, named the same way the running applications list names things, so the exclusion list can be filled from what the user sees there. One name, because Windows knows it exactly.
  - Verified live: PowerShell is `Windows PowerShell`, Bruno is `Bruno`, and the Proton Pass process is `Proton Pass`. In the running application, with `Windows PowerShell` written into the exclusion list, a copy from PowerShell never reached the history, and after the list was edited back it did, without a restart.
  - **A finding, now fixed.** Every .NET copy failed with `CLIPBRD_E_CANT_OPEN` while Dango was running, after a second of retries in the copying application. A .NET application announces delayed-rendered formats and then, on the same thread and without pumping messages, flushes them for real. Reading in between asked that thread to render while it was stuck retrying to open a clipboard we held. The owner is now asked to answer a no-op message before anything is read, and cold PowerShell copies then succeed where every one had failed. The bare spike hit the same failure once in three, so it is the read pattern, not this application.

## 7. The extension

- [x] 7.1 Declare the manifest with its command, its service, and its preferences for the bounds and the exclusion list, and test that it validates
- [x] 7.2 Implement the history command pushing the list newest first with launcher-side filtering, tested against a fake store
- [x] 7.3 Show text entries on one line with whitespace collapsed, with a test over multi-line and indented text
- [x] 7.4 Show image entries with a thumbnail of themselves
  - The entry's own file is the thumbnail, which is the only way to tell one copied image from another. Confirmed live: copying a PNG writes the file and the row points at it.
- [x] 7.5 Put the chosen entry back on the clipboard and hide the launcher, tested for both content types
- [x] 7.6 Add the remove action, leaving the list open, with a test
  - `ActionOutcome` had no way to leave the user where they were: it could hide, copy, or fail. Adds `Replaced`, which hands back the rebuilt view, so clearing several entries is not a chore of reopening the history between each one.
- [x] 7.7 Render the empty state when nothing has been copied, with a test
- [x] 7.8 Grow the list row enough for a thumbnail to be legible, checked by eye on both platforms
  - Rows are 56px with a 32px icon in both lists. The two lists had drifted apart, so the row was extracted into one shared component and the pushed list rebuilt on the same `Command` primitive root search uses, rather than the hand-rolled list it had. macOS looks right.
  - Windows: checked by screenshot. A copied 640x400 image is recognisable at that size, and a multi-line copy shows on one line.

## 8. Verification

- [x] 8.1 Walk every scenario in the two spec files on macOS
  - Walked and passing: text and images are recorded, copying the same thing again reorders rather than duplicates, entries restore to the clipboard, removing one leaves the list open, the empty state explains itself, the history survives a restart in order, both bounds hold and discard oldest first, an excluded application's clipboard never reaches the history, and the list narrows as the user types.
  - Findings along the way, all fixed: an entry larger than the whole size ceiling used to empty the history; the pushed list had drifted from root search in row height, icon size and match highlighting; popping a view stranded keyboard focus; and the action panel's buttons took focus for good on a click.
- [x] 8.2 Walk every scenario in the two spec files on Windows
  - Walked and passing on a release build: text and images are recorded from other applications, copying the same thing again reorders rather than duplicates, a file list is ignored, content carrying either exclusion format never reaches the history, an excluded application's copy never reaches the history and the list is honoured from a database edit without a restart, restoring an entry puts it back and hides the launcher without reordering the history, removing one leaves the list open, an image whose file has gone reports `that entry is no longer there` and its row is removed, the empty state shows, the history survives a restart in order, both bounds hold and discard oldest first, an oversized item is refused, the migration adds its table to an existing database without touching the others, and the list narrows as the user types.
  - Findings: the .NET copy failure under 6.4, fixed. Two limitations recorded rather than fixed: a copy with no owner window or from a process that exits at once cannot be attributed and is recorded, and a writer that exits with only delayed-rendered formats on the clipboard leaves nothing anyone can read, so nothing is recorded. One rough edge left alone: after an entry whose file has gone is chosen, the row is removed but the open list still shows it until it is reopened.
- [x] 8.3 Confirm activation still meets the 80ms budget on release builds on both platforms, with the watcher running
  - macOS: **passes**. 46 activations, median 41.9ms, p90 49.5ms, max 70.0ms, none over 80ms, including copying a large image and opening the launcher immediately after.
  - It failed before the move to `clipboard-rs`: one activation in 27 reached 95.9ms under exactly that contention. The rework removed the cause rather than the symptom. Nothing polls any more, so there is no thread waking four times a second to contend with; the crate delivers a change event instead.
  - Windows: **passes**. 47 activations on a release build with the watcher running, median 22.7ms, p90 25.4ms, max 32.6ms, none over 80ms, with a 3000x2000 image copied before every eighth one and recorded as a 120 to 200KB PNG meanwhile.
- [x] 8.4 Confirm root search stays responsive while a large image is recorded, on both platforms
  - macOS: confirmed by hand. Root search stayed responsive to typing. The activation budget during the same contention did not, which is group 8.3.
  - Windows: a query typed straight after a 3000x2000 image was copied rendered its results normally.
- [x] 8.5 Confirm the history survives a restart with its order intact, on both platforms
  - macOS: confirmed live. Three entries recorded, Dango stopped and started, and all three come back in the same order.
  - Windows: confirmed live twice, once with eight entries and once after the final build.
- [ ] 8.6 Confirm a password manager's clipboard never reaches the history, on both platforms
  - macOS: confirmed live against Proton Pass itself, in the running application, after the move to `clipboard-rs` changed the whole read path. The password does not reach the history.
  - Also confirmed earlier with an arbitrary application on the list, and that the list is honoured from a database edit without a restart.
  - Windows: **not closed.** Proton Pass was locked behind the account password on the test machine and could not be made to copy anything. Everything around it is confirmed: an arbitrary application on the list keeps its copies out of the running application, the list is honoured from a database edit without a restart, both exclusion formats keep content out unread, an Electron application leaves its own process as the clipboard owner, and the Proton Pass process identifies as `Proton Pass`. What remains is one copy of a password from an unlocked Proton Pass with `cargo run --example clipboard_spike` watching, or the application running, to see the owner come back as `Proton Pass` and nothing recorded.
- [x] 8.7 Confirm the bounds hold by copying past the entry limit and past the size ceiling, on both platforms
  - macOS: confirmed live against the running application, with the bounds written straight into the database, which also proves preferences are read per change rather than at startup.
  - The entry bound holds exactly: set to 5, eight copies leave the five newest.
  - The size ceiling discards oldest first: four images against a 12,582 byte ceiling leave 8,967 bytes.
  - **A finding, now fixed.** An entry larger than the whole ceiling used to empty the history, because trimming discards oldest first until the newest fits and for something oversized that only ends at zero. One large copy cost the user everything else they had copied. Such an entry is now refused outright, the way one over the per-entry ceiling already was, and the rest of the history is untouched. Confirmed live.
  - Windows: confirmed live the same way. The entry bound set to 5 leaves the five newest of eight, and deletes the discarded image's file. Four 6,593 byte images against a 12,582 byte ceiling leave one. An image over a 1,048 byte per-entry ceiling is refused with the history untouched.
- [x] 8.8 Confirm recording while searching does not disturb the result list, on both platforms
  - macOS: confirmed by hand. Copying from another application while a query was up left the result list alone.
  - Windows: confirmed by screenshot before and after a copy from another application, with the query and its results unchanged and the copy recorded.
