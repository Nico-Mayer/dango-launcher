## 1. Spike: Windows system control

- [ ] 1.1 Lock the workstation and confirm the session locks with the launcher already hidden
- [ ] 1.2 Put the machine to sleep and confirm it wakes with Dango still resident and the shortcut still registered
- [ ] 1.3 Read the recycle bin item count and empty it, and confirm the count is correct before and zero after
- [ ] 1.4 Enumerate running applications and confirm the list carries a name, an icon, and a way to address each one, with Dango absent
- [ ] 1.5 Ask one application to close and confirm a well-behaved app exits while one with unsaved work prompts instead
- [ ] 1.6 If any of the above does not work, revisit the design before building on it

## 2. Command host and invocation

- [x] 2.1 Define the command trait and the invocation context, and prove with a unit test that a built-in command can be invoked through it
- [x] 2.2 Define the output sink carrying view trees and no-view outcomes, with a test that a command can push more than one tree
- [x] 2.3 Add the host trait with the built-in native implementation, and test that a command declaring an unknown host is refused rather than run
- [x] 2.4 Resolve a command by fully qualified identity, with a test that two extensions declaring the same unqualified identifier do not collide
- [x] 2.5 Reject an invocation of a command whose extension is disabled, and test that the caller gets an unavailable result rather than a panic
- [x] 2.6 Add the generation counter so a newer invocation supersedes an older one, with a test that stale output is dropped
- [x] 2.7 Isolate a failing command so its error surfaces as a failure outcome and the launcher stays usable, with a test over a panicking command

## 3. Wiring invocation to the frontend

- [ ] 3.1 Add the invoke command on the Tauri boundary and confirm from a manual run that choosing a command result reaches the backend
- [ ] 3.2 Emit view trees on `dango://render` and confirm the existing frontend stack renders one without frontend changes
- [ ] 3.3 Return no-view outcomes so success hides the launcher and failure keeps it open with the message, confirmed by running both cases
- [x] 3.4 Generalise `run_action` to dispatch by owning extension, and test that two extensions' results are each handled by their owner
- [ ] 3.5 Drop the hardwired `ApplicationsExtension` lookup and confirm application results still launch, reveal, and copy
- [ ] 3.6 Discard output from an abandoned invocation when the launcher hides, confirmed by dismissing during a deliberately slow command

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

- [ ] 6.1 Lock the screen, verified by invoking it on a real desktop
  - Implemented over the login window's own `CGSession -suspend`. Not invoked here: it would lock the screen out from under the session.
- [ ] 6.2 Sleep the machine, verified by invoking it on a real desktop
  - Implemented over `pmset sleepnow`. Not invoked here for the same reason.
- [ ] 6.3 Read the trash item count and empty it, verified against a trash with known contents
  - Counting is verified live. Emptying is implemented but not run: it would permanently delete whatever is actually in the trash.
  - Reading `~/.Trash` directly turned out to be blocked by Full Disk Access, so both operations go through Finder, which already has it. The first call prompts once for permission to control Finder.
- [x] 6.4 List running applications with names and icons, verified against what the dock and the app switcher show
  - Live test confirms the list is non-empty, carries an addressable id, and excludes Dango. Only regular applications appear, so agents and daemons are left out.
- [x] 6.5 Ask an application to quit, verified against one that exits cleanly and one holding an unsaved document
  - Live test quits Calculator and confirms the process is gone, and that quitting something already exited reports it rather than failing silently. The unsaved-document case is left to the walkthrough, since it needs a human to answer the prompt.

## 7. System extension: Windows

- [ ] 7.1 Lock the workstation, verified by invoking it on a real desktop
- [ ] 7.2 Sleep the machine, verified by invoking it on a real desktop
- [ ] 7.3 Read the recycle bin item count and empty it, verified against a bin with known contents
- [ ] 7.4 List running applications with names and icons, verified against what the taskbar and Alt-Tab show
- [ ] 7.5 Ask an application to close, verified against one that exits cleanly and one holding an unsaved document

## 8. Verification

- [ ] 8.1 Walk every scenario in the three spec files on macOS
- [ ] 8.2 Walk every scenario in the three spec files on Windows
- [ ] 8.3 Confirm activation still meets the 80ms budget on release builds on both platforms, with the system extension loaded
- [ ] 8.4 Confirm the view stack pops back to root search on Escape and clears on hide, on both platforms
- [ ] 8.5 Confirm disabling the `system` extension removes its commands from search without a restart, on both platforms
- [ ] 8.6 Confirm application results still launch, reveal, and copy after `run_action` stopped being hardwired, on both platforms
