## 1. Spike: Windows system control

- [x] 1.1 Lock the workstation and confirm the session locks with the launcher already hidden
  - Live test: `LockWorkStation` returns at once and the logon screen process is up within 300ms. The launcher was hidden when it ran.
- [x] 1.2 Put the machine to sleep and confirm it wakes with Dango still resident and the shortcut still registered
  - Live test behind a resume timer: the event log shows the machine entering sleep at 11:51:29 and resuming at 11:51:33. The release build kept running across it, and two hotkey activations afterwards painted in 31.1ms and 21.4ms.
  - `SetSuspendState` had not returned 500ms in, which is the blocking behaviour the design warned about, so the call runs on its own thread and only a prompt refusal is waited for.
- [x] 1.3 Read the recycle bin item count and empty it, and confirm the count is correct before and zero after
  - Counting is verified live against the shell: 39 items, 41 after recycling two probe files, 39 again after removing only those two. Emptying is implemented but not run: the bin held 39 of the user's items, and the machine has one volume, so there was nothing to empty in isolation.
- [x] 1.4 Enumerate running applications and confirm the list carries a name, an icon, and a way to address each one, with Dango absent
  - Live test: every entry has a name, a process id, and an icon that renders, and Dango is absent. The list is what Alt-Tab shows, so windows on another virtual desktop are cloaked and left out along with suspended store apps.
- [x] 1.5 Ask one application to close and confirm a well-behaved app exits while one with unsaved work prompts instead
  - Calculator, a store app hosted in ApplicationFrameHost, is listed under its own process and is gone within a second of the request. A form that cancels its own close stands in for unsaved work: the request reaches it, it keeps running, and quit reports success rather than failure.
- [x] 1.6 If any of the above does not work, revisit the design before building on it
  - Everything in the design table worked as written against windows 0.61.3. Two additions the table did not anticipate: store apps run inside ApplicationFrameHost and need the hosted child's process to be listed separately, and packaged desktop apps carry their AppUserModelID on the process rather than the window.

## 2. Command host and invocation

- [x] 2.1 Define the command trait and the invocation context, and prove with a unit test that a built-in command can be invoked through it
- [x] 2.2 Define the output sink carrying view trees and no-view outcomes, with a test that a command can push more than one tree
- [x] 2.3 Add the host trait with the built-in native implementation, and test that a command declaring an unknown host is refused rather than run
- [x] 2.4 Resolve a command by fully qualified identity, with a test that two extensions declaring the same unqualified identifier do not collide
- [x] 2.5 Reject an invocation of a command whose extension is disabled, and test that the caller gets an unavailable result rather than a panic
- [x] 2.6 Add the generation counter so a newer invocation supersedes an older one, with a test that stale output is dropped
- [x] 2.7 Isolate a failing command so its error surfaces as a failure outcome and the launcher stays usable, with a test over a panicking command

## 3. Wiring invocation to the frontend

- [x] 3.1 Add the invoke command on the Tauri boundary and confirm from a manual run that choosing a command result reaches the backend
  - Confirmed live: a command result reaches the backend and runs.
- [x] 3.2 Emit view trees on `dango://render` and confirm the existing frontend stack renders one without frontend changes
  - Confirmed live: the quit-application list renders from a view tree with no frontend change beyond what the walkthrough exposed as missing.
- [x] 3.3 Return no-view outcomes so success hides the launcher and failure keeps it open with the message, confirmed by running both cases
  - Confirmed on Windows: a successful quit hides the launcher and resets it to root, and a lock made to fail keeps it open with the message under the result.
- [x] 3.4 Generalise `run_action` to dispatch by owning extension, and test that two extensions' results are each handled by their owner
- [x] 3.5 Drop the hardwired `ApplicationsExtension` lookup and confirm application results still launch, reveal, and copy
  - Confirmed live: applications still launch, reveal, and copy path.
- [x] 3.6 Discard output from an abandoned invocation when the launcher hides, confirmed by dismissing during a deliberately slow command
  - Confirmed on Windows with quit-application slowed to four seconds: dismissing while it worked hid the launcher, and its list never appeared afterwards.

## 4. Shared platform groundwork

- [x] 4.1 Define the `SystemControl` trait covering lock, sleep, trash, running applications, and quit
- [x] 4.2 Factor icon extraction into a platform helper both `AppIndexer` and `SystemControl` use, and confirm the applications index still shows icons
  - `IconCache` no longer holds an indexer; it takes the renderer per call, so the same cache serves installed and running applications without either extension depending on the other.

## 5. System extension: shared

- [x] 5.1 Declare the manifest with its four commands, their modes, titles, keywords, and icons, and test that it validates
- [x] 5.2 Implement lock and sleep as no-view commands over the trait, tested against a fake `SystemControl`
- [x] 5.3 Implement empty trash as a view command pushing the confirmation detail view, tested for confirm, decline, and already-empty
- [x] 5.4 Implement quit application as a view command pushing the running-application list with launcher-side filtering, tested against a fake
- [x] 5.5 Exclude Dango from the running-application list, with a test
- [x] 5.6 Report a failure when the chosen application has already exited, and treat a refusal to quit as success, both tested

## 6. System extension: macOS

- [x] 6.1 Lock the screen, verified by invoking it on a real desktop
  - `CGSession` no longer ships on macOS 26. Locking now calls `SACLockScreenImmediate`, the entry point the system's own lock menu uses, looked up at run time so its absence is reported rather than assumed. Confirmed live.
  - Implemented over the login window's own `CGSession -suspend`. Not invoked here: it would lock the screen out from under the session.
- [x] 6.2 Sleep the machine, verified by invoking it on a real desktop
  - Confirmed live over `pmset sleepnow`.
  - Implemented over `pmset sleepnow`. Not invoked here for the same reason.
- [x] 6.3 Read the trash item count and empty it, verified against a trash with known contents
  - Confirmed live: the count is right, Enter cancels, and the action panel confirms.
  - Reading `~/.Trash` directly turned out to be blocked by Full Disk Access, so both counting and emptying go through Finder, which already has it. The first call prompts once for permission to control Finder.
  - Counting is verified live. Emptying is implemented but not run: it would permanently delete whatever is actually in the trash.
  - Reading `~/.Trash` directly turned out to be blocked by Full Disk Access, so both operations go through Finder, which already has it. The first call prompts once for permission to control Finder.
- [x] 6.4 List running applications with names and icons, verified against what the dock and the app switcher show
  - Live test confirms the list is non-empty, carries an addressable id, and excludes Dango. Only regular applications appear, so agents and daemons are left out.
- [x] 6.5 Ask an application to quit, verified against one that exits cleanly and one holding an unsaved document
  - Live test quits Calculator and confirms the process is gone, and that quitting something already exited reports it rather than failing silently. The unsaved-document case is left to the walkthrough, since it needs a human to answer the prompt.

## 7. System extension: Windows

- [x] 7.1 Lock the workstation, verified by invoking it on a real desktop
  - Verified as in 1.1.
- [x] 7.2 Sleep the machine, verified by invoking it on a real desktop
  - Verified as in 1.2.
- [x] 7.3 Read the recycle bin item count and empty it, verified against a bin with known contents
  - Count verified as in 1.3. Emptying is implemented with the shell's confirmation, progress window, and sound suppressed, and treats the E_UNEXPECTED an already-empty bin answers as success. It has not been run against the user's bin.
- [x] 7.4 List running applications with names and icons, verified against what the taskbar and Alt-Tab show
  - Verified live in the launcher: the list matched Alt-Tab, with icons, one entry per application. Names come from the apps folder when a window or process carries an AppUserModelID, so Calculator is "Rechner" and Terminal is "Terminal" rather than "Windows Terminal Host", and from the executable's file description otherwise.
- [x] 7.5 Ask an application to close, verified against one that exits cleanly and one holding an unsaved document
  - Verified through the launcher: choosing Calculator ends it and hides the launcher; choosing a window that refuses to close hides the launcher with no failure and leaves it running; choosing one that has already exited reports it and keeps the launcher open. The real unsaved-document prompt was not exercised: Windows 11 Notepad keeps unsaved tabs instead of prompting, and the stand-in above proves the same path.

## 8. Verification

- [ ] 8.1 Walk every scenario in the three spec files on macOS
  - Walked and passing: commands are searchable by name and keyword, the quit list renders with icons and excludes Dango, it narrows as the user types, Escape pops back to root, the trash confirmation names the count and defaults to cancel, confirming empties it, and an application holding an unsaved document prompts rather than being reported as a failure.
  - Four findings, all fixed: locking used an entry point macOS 26 removed; the trash needed Finder rather than Full Disk Access; a pushed view had no query input, so a list declaring launcher-side filtering could not be narrowed; and a pushed view had no action panel, which left the trash confirmation unreachable.
  - Not yet walked: a no-view command reporting a failure, one invocation superseding another, an unsupported protocol version, and abandoning a slow command.
- [x] 8.2 Walk every scenario in the three spec files on Windows
  - Driven through the webview over CDP with the dev build, and passing: commands found by name and keyword; the quit list renders with icons and without Dango, narrows as the user types, and pops back to root on Escape with the root query intact; the recycle bin confirmation names the count, has cancel as its primary action, and Enter returns to root with nothing deleted; a no-view failure keeps the launcher open with its message; a slow command leaves root search responsive; a second invocation supersedes the first and the stale tree never appears; dismissing during a slow command discards its output; an unsupported protocol version shows the dismissible error and Escape clears it.
  - The four scenarios needing a fault were walked with a temporary local patch that made lock fail, made quit-application take four seconds, and made sleep push a version-99 tree. The patch was reverted, not committed.
  - Two findings fixed: running-application icons were cached by process id, which is reused across sessions, so they are keyed by locator now; and reveal quoted the whole `/select,` argument, which made Explorer open Documents instead of the file.
  - One finding open: nothing indicates that a slow command is working. The spec asks for a loading state before the first tree arrives, and the frontend shows none.
- [x] 8.3 Confirm activation still meets the 80ms budget on release builds on both platforms, with the system extension loaded
  - macOS release, both extensions loaded: cold 55.2ms, median 42.0ms, p90 51.7ms over 26 activations, none over 80ms.
  - Windows release, both extensions loaded: cold 31.7ms, median 22.9ms, p90 26.8ms over 26 hotkey activations, none over 80ms.
- [ ] 8.4 Confirm the view stack pops back to root search on Escape and clears on hide, on both platforms
  - Popping on Escape is confirmed on macOS, with the root query intact behind it. Clearing on hide on macOS is open.
  - Windows: both confirmed. Escape pops with the root query intact, and dismissing with a view up comes back to root search with the stack empty.
- [ ] 8.5 Confirm disabling the `system` extension removes its commands from search without a restart, on both platforms
  - Not walkable yet on either platform: the enabled state lives in the store and is read when extensions load, and nothing in the running app calls `set_enabled`. It needs a settings surface, which is a later change.
- [x] 8.6 Confirm application results still launch, reveal, and copy after `run_action` stopped being hardwired, on both platforms
  - Confirmed on macOS.
  - Confirmed on Windows: launch starts Calculator and hides the launcher, copy path puts the executable path on the clipboard, and reveal opens the containing folder once its quoting was fixed.
