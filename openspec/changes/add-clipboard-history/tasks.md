## 1. Spike: reading the clipboard on both platforms

- [ ] 1.1 On macOS, read the change counter, the text, and an image, and confirm the counter moves exactly once per copy
- [ ] 1.2 On Windows, read the sequence number, the text, and an image, and confirm it moves exactly once per copy
- [ ] 1.3 On macOS, copy from a password manager and confirm the concealed marker is present before any content is read
- [ ] 1.4 On Windows, copy from a password manager and confirm the exclusion formats are present before any content is read
- [ ] 1.5 On Windows, identify the copying application through the clipboard owner, and on macOS through the frontmost application, and record how often macOS attributes wrongly
- [ ] 1.6 Measure what reading a large image costs, to size the poll interval against it
- [ ] 1.7 If any of the above does not work, revisit the design before building on it

## 2. Preference storage

- [ ] 2.1 Add typed get and set on the store, scoped by extension and optional command, tested against an in-memory store
- [ ] 2.2 Return the declared default for a preference that was never set, with a test
- [ ] 2.3 Return the declared default when a stored value no longer parses as its declared type, with a test that the extension still loads
- [ ] 2.4 Keep extension-level and command-level values with the same key apart, with a test
- [ ] 2.5 Hand an extension a reader bound to its own manifest, and test that it cannot read another extension's values
- [ ] 2.6 Confirm a value survives a restart, with a test over a reopened store

## 3. History storage

- [ ] 3.1 Add the migration for the history as a `local_` table, and confirm it applies to an existing database without touching the other tables
- [ ] 3.2 Record a text entry and read the history back newest first, with a test
- [ ] 3.3 Record an image entry as a file plus a row, and confirm the row points at a file that exists
- [ ] 3.4 Move an existing entry to newest instead of inserting a duplicate, with a test over the same content copied twice
- [ ] 3.5 Enforce the entry-count bound, discarding oldest first, with a test
- [ ] 3.6 Enforce the total-size bound, discarding oldest first, with a test
- [ ] 3.7 Refuse an entry over the per-entry ceiling without disturbing the history, with a test
- [ ] 3.8 Delete an image's file when its entry is discarded or removed, with a test that the file is gone
- [ ] 3.9 Remove an entry whose file has vanished and report the failure, with a test

## 4. The watcher

- [ ] 4.1 Define the `ClipboardSource` trait covering the counter, the markers, the owning application, and reading text and images
- [ ] 4.2 Implement the watcher service polling the counter, tested against a fake source driven by hand
- [ ] 4.3 Check the markers and the exclusion list before reading any content, with a test that excluded content is never read
- [ ] 4.4 Read the exclusion list per change so editing it takes effect without a restart, with a test
- [ ] 4.5 Ignore the write Dango itself makes when restoring an entry, with a test
- [ ] 4.6 Stop the watcher cleanly when the extension is disabled, with a test
- [ ] 4.7 Confirm the watcher runs off the activation path, with a test that no clipboard work happens on the activation path

## 5. Platform: macOS

- [ ] 5.1 Read the change counter, verified against copying by hand
- [ ] 5.2 Detect the concealed and auto-generated markers, verified against a password manager
- [ ] 5.3 Read text and images, verified against copying each by hand
- [ ] 5.4 Name the frontmost application as the copying application, verified against copying from a known application

## 6. Platform: Windows

- [ ] 6.1 Read the sequence number, verified against copying by hand
- [ ] 6.2 Detect the exclusion formats, verified against a password manager
- [ ] 6.3 Read text and images, verified against copying each by hand
- [ ] 6.4 Name the clipboard owner as the copying application, verified against copying from a known application

## 7. The extension

- [ ] 7.1 Declare the manifest with its command, its service, and its preferences for the bounds and the exclusion list, and test that it validates
- [ ] 7.2 Implement the history command pushing the list newest first with launcher-side filtering, tested against a fake store
- [ ] 7.3 Show text entries on one line with whitespace collapsed, with a test over multi-line and indented text
- [ ] 7.4 Show image entries with a thumbnail of themselves
- [ ] 7.5 Put the chosen entry back on the clipboard and hide the launcher, tested for both content types
- [ ] 7.6 Add the remove action, leaving the list open, with a test
- [ ] 7.7 Render the empty state when nothing has been copied, with a test
- [ ] 7.8 Grow the list row enough for a thumbnail to be legible, checked by eye on both platforms

## 8. Verification

- [ ] 8.1 Walk every scenario in the two spec files on macOS
- [ ] 8.2 Walk every scenario in the two spec files on Windows
- [ ] 8.3 Confirm activation still meets the 80ms budget on release builds on both platforms, with the watcher running
- [ ] 8.4 Confirm root search stays responsive while a large image is recorded, on both platforms
- [ ] 8.5 Confirm the history survives a restart with its order intact, on both platforms
- [ ] 8.6 Confirm a password manager's clipboard never reaches the history, on both platforms
- [ ] 8.7 Confirm the bounds hold by copying past the entry limit and past the size ceiling, on both platforms
- [ ] 8.8 Confirm recording while searching does not disturb the result list, on both platforms
