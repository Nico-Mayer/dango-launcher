## 1. Spike: Windows application enumeration

- [x] 1.1 Enumerate the shell applications folder and confirm both packaged and conventional desktop applications are returned with display names and launch identifiers
- [x] 1.2 Extract an icon for one of each kind at the size the result list needs
- [x] 1.3 Launch one of each kind by its identifier, and confirm an already-running application is brought forward rather than started twice
- [x] 1.4 Measure how long a full enumeration takes on a typical machine
- [x] 1.5 If any of the above does not work, revisit the design before building on it

## 2. Local store

- [ ] 2.1 Open or create the database in the per-user application data directory at startup
- [ ] 2.2 Add the forward-only migration runner, applying each migration at most once and refusing to run against a newer schema
- [ ] 2.3 Add the first migration covering extension enabled state, preferences, frecency, and the application index
- [ ] 2.4 Apply the syncable record convention of UUID identifier, `updated_at`, and soft delete to the tables that need it, and keep machine-local tables separate
- [ ] 2.5 Handle a corrupt database by starting empty, preserving the file, and reporting it
- [ ] 2.6 Confirm no database access sits on the activation path

## 3. Extension model

- [ ] 3.1 Define the manifest type with `manifestVersion`, identifier, name, and icon, and reject unsupported versions without stopping other extensions
- [ ] 3.2 Define the four contribution types and reject a manifest that contributes none
- [ ] 3.3 Define command declarations with identifier, title, mode, subtitle, icon, keywords, and alias, and resolve fully qualified identities
- [ ] 3.4 Define the typed preference schema with defaults and required values
- [ ] 3.5 Implement the activate and deactivate lifecycle so it registers and unregisters commands, root providers, and services
- [ ] 3.6 Persist enabled state and restore it at startup
- [ ] 3.7 Isolate failures so a broken extension cannot stop the launcher opening or block other extensions

## 4. Command registry

- [ ] 4.1 Build the registry holding commands from all enabled extensions, keyed by fully qualified identity
- [ ] 4.2 Record which host executes each command, with only the built-in native host implemented
- [ ] 4.3 Detect and report identifier collisions between extensions
- [ ] 4.4 Support live registration and unregistration so enabling and disabling needs no restart

## 5. View protocol

- [ ] 5.1 Define the protocol types for list, detail, and form views with `protocolVersion`
- [ ] 5.2 Define the action panel, primary action, and per-action shortcuts
- [ ] 5.3 Define the declared filtering ownership, loading state, and empty state
- [ ] 5.4 Define the no-view command result carrying success or failure
- [ ] 5.5 Define the invocation channel carrying full view trees from a command to the frontend
- [ ] 5.6 Generate or hand-maintain the matching TypeScript types and check them in CI

## 6. Search pipeline

- [ ] 6.1 Define the root items provider trait, async and cancellable
- [ ] 6.2 Implement per-keystroke cancel and restart with no debounce and at most one query in flight
- [ ] 6.3 Enforce the 50ms provider budget and merge late results as they arrive without blocking
- [ ] 6.4 Handle a provider that errors or never returns, without leaking the abandoned work
- [ ] 6.5 Stream partial results to the frontend as providers answer
- [ ] 6.6 Bound the result list sent to the frontend

## 7. Ranking

- [ ] 7.1 Add fuzzy subsequence matching over title, keywords, and alias, returning a score and matched character positions
- [ ] 7.2 Add the frecency score, updated on launch and decayed by age, persisted in the store
- [ ] 7.3 Combine match quality and frecency into the final ordering, with exact alias match ranked first
- [ ] 7.4 Implement empty-query behaviour showing bounded frecent items, and a sensible first-run list
- [ ] 7.5 Add tests for ordering, including equal-match-different-frecency and equal-frecency-different-recency
- [ ] 7.6 Benchmark ranking against 2000 candidates and confirm the 30ms budget

## 8. Applications extension: shared

- [ ] 8.1 Define the `AppIndexer` trait and the indexed application record
- [ ] 8.2 Declare the extension manifest and its root items provider
- [ ] 8.3 Run indexing in the background at startup, persist results, and keep the launcher usable while it runs
- [ ] 8.4 Add the refresh path so installs and uninstalls are picked up without a restart
- [ ] 8.5 Remove a stale entry when launching it fails because it no longer exists
- [ ] 8.6 Add the reveal in file manager and copy path actions
- [ ] 8.7 Cache extracted icons off the search path and show a placeholder when extraction fails

## 9. Applications extension: macOS

- [ ] 9.1 Discover bundles in the system, user, and system-owned applications directories and their immediate subdirectories
- [ ] 9.2 Read display names, honouring a declared display name over the bundle filename
- [ ] 9.3 Extract icons at the required size
- [ ] 9.4 Launch, bringing an already-running application to the foreground instead of starting a second instance
- [ ] 9.5 Watch the applications directories for changes

## 10. Applications extension: Windows

- [ ] 10.1 Enumerate applications through the shell so packaged and conventional applications both appear
- [ ] 10.2 Capture display names and launch identifiers, and filter out uninstall and maintenance entries
- [ ] 10.3 Extract icons at the required size for both kinds
- [ ] 10.4 Launch by identifier, bringing an already-running application to the foreground
- [ ] 10.5 Refresh the index when the installed application set changes

## 11. Frontend

- [ ] 11.1 Replace M0's placeholder with the query input and result list
- [ ] 11.2 Render the protocol: list, detail, and form views
- [ ] 11.3 Key list items by identifier so full-tree replacement preserves selection and scroll without flicker
- [ ] 11.4 Implement keyboard navigation, Enter for the primary action, and the action panel with its shortcuts
- [ ] 11.5 Implement the view stack with push, pop on Escape, and hide on Escape at root
- [ ] 11.6 Clear the stack to root on the reset-on-hide signal from M0
- [ ] 11.7 Render loading and empty states, and highlight matched characters in results
- [ ] 11.8 Handle an unsupported `protocolVersion` with a dismissible error
- [ ] 11.9 Render the outcome of a no-view command, hiding on success and staying open on failure

## 12. Verification

- [ ] 12.1 Walk every scenario in the five spec files on macOS
- [ ] 12.2 Walk every scenario in the five spec files on Windows
- [ ] 12.3 Confirm activation still meets the 80ms budget on release builds on both platforms, with the index loaded
- [ ] 12.4 Confirm a deliberately slow root items provider does not delay the result list
- [ ] 12.5 Use the launcher as the daily application launcher for a week on Windows and record what the contract got wrong
