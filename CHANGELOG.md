# Changelog

All notable changes to Dango. Generated from commit history at release time.

## [6.0.0] - 2026-11-03

### Features

- **config:** Move schemas into .schemas folder
- **ai:** [**breaking**] Read provider keys from auth.toml
- **snippets:** [**breaking**] Store snippets and quicklinks as TOML

### Fixes

- **snippets:** Keep field comments and unknown keys
- **app:** Keep startup config error on its lines

### Maintenance

- Drop leftover JSON config file references
## [5.0.0] - 2026-11-03

### Features

- **config:** Parse the config file as toml
- **config:** Read config.toml instead of config.json
- **config:** Point at the schema with a #:schema line
- **config:** [**breaking**] Give each built-in its own table
- **config:** [**breaking**] Turn the hyperkey on with enabled
- **config:** [**breaking**] List selection exclusions as an array
- **config:** [**breaking**] Name the grace period reset-after-seconds
- **text:** Lend the foreground to read a selection
- **windows:** Copy selection with Ctrl+Insert

### Fixes

- **config:** List a built-in table only once
- **windows:** Keep launcher up while foreground is lent
- **windows:** Keep launcher up during a lent read
- **macos:** Hide panel before the copy fallback
- **macos:** Trust only real AX selection answers
- **macos:** Post the copy chord to the frontmost pid
- **macos:** Never ask the system-wide AX element
- **macos:** Skip Dango as the app behind a read

### Refactoring

- **config:** Share clipboard defaults with bounds
- **text:** [**breaking**] Drop selection exclusion list

### Documentation

- Describe config.toml and the schema line
- **config:** Say settings, not preferences

### Maintenance

- **config:** Keep reserved names out of tables
- **config:** Drop the json file test
- **app:** Show failed selection read at once
- **app:** Pin hotkey command run behind the show
- **ui:** Use a current message for toast width
## [4.1.0] - 2026-11-02

### Features

- **log:** Record panics in dango.log
- **log:** Add time and backtrace to panic log
- **config:** Add claude and pi provider kinds
- **ai:** Run agent providers without model or key
- **ai:** Give agent programs a quiet limit
- **ai:** Add claude and pi argument tables
- **ai:** Find agent programs in a folder list
- **ai:** Read PATH from env output
- **platform:** List folders agent CLIs install to
- **platform:** Spawn agent CLIs hidden and contained
- **ai:** Run agent CLIs as a completions client
- **ai:** Send agent kinds to the agent client
- **ai:** Stop agent CLIs when Dango quits

### Fixes

- **windows:** Retry icons while the shell is busy
- **clipboard:** Render file icons after showing history
- **system:** Render running app icons after the list
- **ui:** Keep a command's view after an action
- **ai:** Trim the answer before delivering it
- **examples:** Update walkthrough Request fields
- **ai:** Allow agent lookup code before its caller
- **ai:** Keep macOS agent runs from misreporting
- **ai:** End agent runs when the program exits
- **platform:** Read macOS login env as bytes

### Refactoring

- Move icon cache out of applications
- **windows:** Use the crate's E_PENDING
- **ai:** Drop dead code allow on agent modules

### Documentation

- **agents:** Note tray PATH and hidden console traps
- **agents:** Note synthetic hotkey focus limit
- **agents:** Note open env and grant relaunch

### Maintenance

- **clipboard:** Accept fallback file icon
- **clipboard:** Render file icons without the shell
- Keep symbols in release builds
- **ai:** Widen quiet limit margin in stream test
- **deps:** Bump windows to 0.62
## [4.0.0] - 2026-11-02

### Features

- **mcp:** Add press, snapshot and restart tools
- **hyperkey:** [**breaking**] Drop the hyperkey.key setting
- **permissions:** Name hyperkey in Accessibility warning
- **icons:** Add file icon
- **clipboard:** Record and restore copied files
- **search:** Name the owning extension in root rows

### Fixes

- **mcp:** Fail launch while another Dango runs
- **ai:** Never send a key to a local provider
- **ranking:** Clear pinned flag on typed queries
- **hotkeys:** Name the taken launcher chord
- **ai:** Skip commands whose prompt can't parse
- **ui:** Announce kbd and tag text once
- **windows:** Reinstall keyboard hooks on a timer
- **windows:** Hold Ctrl around synthetic chords
- **windows:** Log when hook timer fails
- **windows:** Hold Ctrl only after the chord key
- **macos:** Report empty selection, skip Cmd+C
- **macos:** Bound direct selection read
- **macos:** Restart hyperkey on permission change
- **macos:** Report hyperkey permission failures
- **macos:** Skip hyperkey notice for missing grant
- **macos:** No config error for missing hyperkey grant
- **clipboard:** Share file icons by type

### Refactoring

- **hotkeys:** Reuse launcher problem for label
- **ai:** Simplify prompt parse check
- **config:** Drop unused config write-back
- **config:** Drop unused serialize path
- **text:** Share direct read deadline
- **extension:** [**breaking**] Drop command subtitle

### Documentation

- **agents:** Note OpenSpec CLI tracks latest
- **agents:** Compress AGENTS.md to caveman style
- **agents:** Document dango-ui tools and scopes
- Note silent low-level hook removal
- Note hook swap reorders the chain
- Correct chord hold note
- **extension:** Drop stale persist note
- **spec:** Drop config write-back from schema spec
- Fix stale store comments
- **hyperkey:** Name CapsLock in wording
- Note own bundle ID for macOS permission checks
- Note reopen drops env in permission checks

### Maintenance

- **mise:** Link tool skills into .agents and .claude
- **claude:** Disable the svelte plugin again
- Drop duplicate Slint skill and docs server
- Pin slint-lsp and slint-viewer via mise
- Drop slint-viewer
- Move claude md
- Drop stale skills-lock.json
- Manage agent setup with skillshare
- Add rust-skills and ignore agent folders
- Add new skill
- Mise tasks
## [3.0.3] - 2026-11-01

### Fixes

- **macos:** Tie the login item to the Dango bundle

### Documentation

- Record CI mise and tag cache facts

### Maintenance

- Stop mise run installing every tool
- Drop the Rust cache from releases
- Cancel superseded CI runs
- Skip CI for docs-only pushes
- Save no mise cache from releases
- Run CI for docs changes
## [3.0.2] - 2026-11-01

### Fixes

- **macos:** Show the launcher on reopen
- Name the login entry in autostart errors
## [3.0.1] - 2026-11-01

### Fixes

- **macos:** Keep the instance lock in the data dir

### Refactoring

- Share the data dir with the instance lock

### Documentation

- **config:** Note the hyperkey takes F18 on macOS

### Maintenance

- Let the release task prompt and skip publishing
- Install rust before cargo-packager
## [3.0.0] - 2026-11-01

### Features

- **ui:** Add Theme global following the OS scheme
- **ui:** Add Icon, Kbd, Tag, Accessory, Dot atoms
- **protocol:** [**breaking**] Move the view protocol to version 2
- **protocol:** Add list sections, accessories, keywords
- **protocol:** Add action sections and danger style
- **protocol:** Add empty state icon and actions
- **ui:** [**breaking**] Show status and failures as toasts
- **extensions:** Mark delete actions as danger
- **ui:** Carry the ActionBar context in snapshots
- **ui:** Add the list detail pane and item details
- **ui:** Add the action panel filter
- **ui:** Render detail markdown and metadata
- **ui:** Build form controls on primitives
- **protocol:** Add the grid view
- **protocol:** Add the view filter dropdown
- **shell:** Size the launcher to its content
- **ui:** Bundle the Geist variable fonts
- **ui:** Show at most 50 items in a view
- **config:** Read per-command enabled state
- **extension:** Skip disabled commands on activate
- **extension:** Re-sync commands on config reload
- **hotkeys:** Skip hotkeys of disabled commands
- **ui:** [**breaking**] Refuse Shift or Alt on non-letter keys
- **ui:** Allow Shift or Alt on named keys
- **config:** Accept F13 to F20 in hotkeys
- **config:** Refuse a hotkey without a modifier
- **config:** Let a lone F1 to F20 match the schema
- **snippets:** Refuse a keyword that overlaps another
- **snippets:** Skip keystrokes typed into Dango
- **clipboard:** Store a thumbnail of each image
- **clipboard:** Show the thumbnail in image rows
- **protocol:** Let a detail carry an image and text
- **ui:** Show an image and plain text in a detail
- **store:** Add clipboard source and size columns
- **clipboard:** Record the copying application
- **clipboard:** Store image size and re-copy source
- **clipboard:** Keep a pane-sized image preview
- **clipboard:** Choose the image the pane shows
- **clipboard:** Count lines and characters
- **clipboard:** Detect text kinds and format times
- **clipboard:** Show entries in a detail pane
- **ui:** Pin detail metadata to the bottom
- **games:** Read installed games from Steam
- **icons:** Add gamepad-2
- **games:** List Steam games in root search
- **games:** Refresh the list when Steam changes
- **games:** Launch games through Steam
- **games:** Add a Search Games cover grid
- **opener:** Open a URL in a named browser
- **quicklinks:** Let a quicklink name its browser

### Fixes

- **copy:** Cut failure messages to one toast sentence
- **ui:** Round the ActionBar's bottom corners
- **shell:** Set the component height before resizing
- **ui:** Keep Tab from moving focus off the prompt
- **ui:** Drop the ActionBar icon gap without an icon
- **ui:** Keep list rows clear of the scrollbar
- **ui:** Let root search settle before shrinking
- **apps:** Keep a running app's window maximized
- **apps:** Raise a running app via force_foreground
- **hyperkey:** Stop a lone tap opening Microsoft 365
- **hyperkey:** Refuse a tap key that needs Shift
- **system:** Check the image path before quitting
- **windows:** Forget the target on a null foreground
- **shell:** Remember the target on the first show
- **app:** Reset at once on a deliberate dismiss
- **app:** Keep the hold when hidden for work
- **app:** Do not show a visible launcher again
- **config:** Skip watched files that fail to read
- **snippets:** Refuse saves while file is broken
- **snippets:** Report a save that failed to write
- **snippets:** Write the records file atomically
- **app:** Keep autostart when config is broken
- **snippets:** Derive missing ids from the name
- **snippets:** Note target before selection read
- **text:** Restore the clipboard when a key fails
- **templates:** Keep a snippet's final newline
- **apps:** Write cached icons atomically
- **window:** Keep Make Smaller from growing windows
- **apps:** Raise packaged apps found by process
- **ui:** Clear multiline focus outside a form
- **ui:** Label rows and tiles with their titles
- **hotkeys:** Keep the old launcher chord if refused
- **app:** Report a hotkey for an unavailable command
- **app:** Keep reported problems across reloads
- **app:** Copy through the shared clipboard source
- **app:** Report extension load problems on reload
- **app:** Write startup problems to dango.log
- **hotkeys:** Report an invalid launcher hotkey
- **log:** Rotate dango.log at 1 MB
- **updates:** Stop update checks when disabled
- **updates:** Match error texts to the spec
- **ai:** Redact the key in Request's Debug output
- **ai:** Need no key for a server on this machine
- **ai:** Say a refused request was refused
- **ai:** Count thinking as activity for the timeout
- **ai:** Report a bad command value, keep the config
- **hotkeys:** Log a hotkey on a disabled command
- **clipboard:** Log failures to dango.log
- **clipboard:** Skip a change with unreadable formats
- **clipboard:** Stop the old watch on a quick restart
- **clipboard:** Delete orphaned images on start
- **clipboard:** Fingerprint with a stable hash
- **text:** Restore files, HTML and RTF after a paste
- **clipboard:** Write CRLF line breaks on Windows
- **quicklinks:** Keep favicon when refetch fails
- **quicklinks:** Give each favicon sweep its own flag
- **shell:** Keep a silent client from blocking launches
- **shell:** Stop caching file icons forever
- **store:** Roll back a failed app reindex
- **ranking:** Trim spaces around the query
- **ranking:** Key usage by extension and item id
- **ai:** Tell server errors apart from refusals
- **app:** Record no use for an action without item
- **ui:** Check trees returned by actions
- **keymonitor:** Treat AltGr characters as text
- **clipboard:** Skip thumbnails for small images
- **clipboard:** Honour CanIncludeInClipboardHistory
- **clipboard:** Count lines and old images right
- **clipboard:** Require a TLD for email addresses
- **applications:** Leave Steam games to games
- **games:** Notice a finished download sooner
- **examples:** Drop the removed cli request field
- **ui:** Keep the cursor after a restored query
- **opener:** Pass browsers by path on Windows
- **snippets:** Skip keys aimed at Dango on macOS
- **shell:** Attach the macOS panel on first show
- **snippets:** Skip keys while the panel is key

### Performance

- **windows:** Stop broadcasting foreground lock change
- **windows:** Cache the foreground app's name
- **app:** Release the host before an action runs
- **snippets:** Read keywords without cloning
- **snippets:** Work out quicklink hosts on load
- **apps:** Skip persisting an unchanged index
- **apps:** Look up cached icons in memory
- **ui:** Skip unchanged form rows
- **app:** Restart the hyperkey only on change
- **ui:** Group a pushed list once per snapshot
- **clipboard:** Read one history entry by id
- **ui:** Keep file icons from one render to the next
- **ui:** Let go of images when the launcher hides

### Refactoring

- **ui:** Replace Palette with Theme
- [**breaking**] Remove extension tints
- [**breaking**] Remove the development build marker
- **ui:** Split the launcher into globals and parts
- **shell:** Pass the resize into render::publish
- **shell:** Keep one tray menu definition
- **snippets:** Reuse templates Rendered
- **text:** Drop unused permission calls
- **apps:** Drop the test-only provider copy
- **ui:** Drop dead open-panel click guards
- **ui:** Delete unused Theme.max-height
- **ui:** Keep layout sizes in one place
- **updates:** Drop DisabledSource
- **ai:** Drop the unused Chunks cleanup hook
- **ui:** Remove dead webview-era seams
- **invocation:** Run built-ins without a host map
- **extension:** Drop serde from manifest types
- **protocol:** Delete unused CommandOutcome
- **ui:** Share the result limit with views
- **clipboard:** Move busy probe into platform
- **platform:** Add is_main_thread
- **platform:** Move OS labels into platform
- **windows:** Share process_aumid with system
- **platform:** Name the inline main-thread check
- **extension:** Drop required preference flag
- **config:** Drop command-scoped preferences
- **protocol:** [**breaking**] Drop command-owned filtering
- Share the refresh wait between extensions
- **snippets:** Keep create and update for tests

### Documentation

- Describe the design system UI in AGENTS.md
- Drop the GitHub issue rule from AGENTS.md
- Match the clippy command to CI
- Keep open hand checks in tasks.md
- Point to the unarchived design-system spec
- List the Windows hand check tools
- Point to the main design-system spec
- **agents:** Note the item limit for every list
- **protocol:** Document the shortcut letter rule
- Note the AltGr chord trap on Windows
- **platform:** Say the macOS check is still open
- Note what DANGO_CONFIG_DIR does not move
- **agents:** Note hand check key limits
- **agents:** Note hk drops the merge state
- Note the Rust toolchain pin

### Maintenance

- **ui:** Check theme contrast in both schemes
- Drop contrast checks from specs and tests
- **ui:** Cover the SearchBar, ActionBar, and toasts
- Vendor the rust-best-practices skill
- Drop the duplicate OpenSpec skills
- Ignore local audit reports
- **ranking:** Pin the clock in a persistence test
- **app:** Cover held command views and hides for work
- **ui:** Cover panel and grid window heights
- **ui:** Cover match underlining with punctuation
- **ui:** Cover the item limit in sections and grids
- **ui:** Render fifty rows of a long pushed list
- Rebuild the resource when the icon changes
- **app:** Cover a command disabled by reload
- **ui:** Rewrap the group doc comment
- **ui:** Refuse a root action's bad shortcut tree
- Optimise image decoding in debug builds
- **ui:** Hide through the shell in the image test
- **deps:** Add steamlocate
- **games:** Filter and launch from the cover grid
- **games:** Drop the duplicate launch check
- Ignore agent worktrees
- Keep the dango-ui MCP server connected
- Pin the Rust toolchain
- Add mise tasks for repeated commands
- Pin the exact Rust patch release
- Check through mise and the toolchain pin
- Keep agent skills only in .claude/skills
- Keep only line tables in dev debug info
- **ui:** Pick a quicklink browser in the form
- **opener:** Drop a test that checks nothing
## [2.0.0] - 2026-10-31

### Features

- **updates:** [**breaking**] Remove the tray install item
- **ui:** Add launcher state machine
- **updates:** Add a disabled update source
- **ui:** Add a Slint shell beside Tauri
- [**breaking**] Make the Slint shell the dango binary
- **updates:** Update from the GitHub releases

### Fixes

- **ui:** Build the macOS event loop with user events
- **ui:** Hide on focus loss through a WinEvent hook
- **ui:** Time activation paint on the redraw event

### Refactoring

- **ui:** Render launcher from Rust snapshots
- Use global-hotkey types in the core
- **snippets:** Use url crate instead of tauri
- Run the app through Core and Shell
- [**breaking**] Remove the Svelte frontend and npm toolchain
- [**breaking**] Move the crate to the repo root

### Documentation

- **agents:** Let agents commit their own work
- **agents:** Record Slint traps found on Windows
- **agents:** Work one platform at a time
- **agents:** Rewrite for the Slint stack
- **readme:** Describe the Slint build and credit Slint
- **agents:** Keep release test runs off the login entry
- **agents:** Limit text input to English and German
- Reword comments that relied on the webview
- Describe packaging and releases

### Maintenance

- **browser:** Drop tests ported to cargo test
- Add Slint docs and app MCP servers
- Ignore local Claude settings
- Add the Slint agent skill
- **scripts:** Add screenshot and key scripts
- **hk:** Drop svelte-check
- Drop Node and the binding diff check
- Switch off releases until native-release-pipeline
- Drop the Tauri and Svelte agent tooling
- Drop the Tauri bundle icons
- Pin cargo-packager in mise
- Configure cargo-packager in Cargo.toml
- Release with cargo-packager
- **updates:** Check the release key signature
- Ignore the .design folder
- Add the ponytail agent skills
## [1.0.0] - 2026-10-31

### Features

- **ai:** [**breaking**] Remove the cli provider kind

### Refactoring

- **config:** Make PerPlatform a string again

### Documentation

- **agents:** Drop tray PATH trap
## [0.6.0] - 2026-10-30

### Features

- **ai:** Search configured dirs for cli programs

### Refactoring

- **config:** Make PerPlatform generic

### Documentation

- **agents:** Note tray PATH and .cmd lookup trap
## [0.5.0] - 2026-10-30

### Features

- **config:** Add launcher.reset-after setting
- **shell:** Hold launcher state after interrupting hide
- **search:** Pin results above an empty query
- **platform:** Check macOS privacy permissions
- **permissions:** Warn about missing permissions
- **updates:** Add self-update extension

### Fixes

- **text:** Stop blaming a missing text path on AX
- **text:** Build the exchange without history
- **macos:** Start key injection on first use
- **search:** Merge providers in registration order

### Refactoring

- Move Runner into invocation

### Documentation

- **roadmap:** Record permission warnings in M7
- Remove roadmap
- Add AGENTS.md as agent source of truth
- **specs:** Replace timing budgets with qualitative rules
- Note local bundling needs the updater key
- Note local macOS builds sign ad-hoc

### Maintenance

- Remove pi agent files
- Drop timing-budget assertions
- Remove tests that prove nothing
- Drop tests of library behaviour and static data
- Cut ActionPanel browser test combinations
- Stop pinning pixels and unread screenshots
- Match AI error variants, not their copy
- Run search tests on paused tokio time
- Cover snippet keyword expansion
- Cover the Windows invisible-border correction
- Generate IPC payload types with ts-rs
- Cover frecency and app index persistence
- **release:** Sign updater and macOS bundles
## [0.4.0] - 2026-10-10

### Features

- **ranking:** Learn typed input per chosen item
- **shell:** Mark development builds in window and tray

### Fixes

- **applications:** Stop dropping Explorer on launch

### Refactoring

- **ranking:** Adopt nucleo-matcher scorer
## [0.3.1] - 2026-10-10

### Fixes

- **config:** Name commands by id in schema
## [0.3.0] - 2026-10-10

### Features

- **system:** Add Open Config File command
- **config:** Generate schema and settings reference

### Maintenance

- Deps bump
## [0.2.0] - 2026-10-10

### Features

- **launcher:** Add autostart config setting

### Fixes

- **release:** Stop non-fix commits bumping version

### Documentation

- Update roadmap
## [0.1.0] - 2026-09-17

### Features

- **launcher:** Add tray shell with hotkey and panel window
- **launcher:** Add windows window behaviour and tray fallbacks
- **launcher:** Add windows app enumeration spike
- **store:** Add sqlite store with forward-only migrations
- **extension:** Add manifest, registry, and lifecycle
- **protocol:** Add versioned view protocol with ts bindings
- **search:** Add cancellable streaming root pipeline
- **ranking:** Add fuzzy matching and frecency ranking
- **applications:** Add shared app index, icons, and actions
- **applications:** Add windows shell indexer
- **frontend:** Wire search pipeline and render results
- **applications:** Add the macos indexer
- **system:** Add command invocation and system commands
- **frontend:** Add an icon provider and fix the pushed view
- **system:** Add the windows system control
- **clipboard:** Add preference, history, and watcher plumbing
- **clipboard:** Add the macos clipboard source
- **clipboard:** Add the history extension
- **clipboard:** Name the clipboard owner on windows
- **templates:** Add the shared placeholder engine
- **protocol:** Make a form submit an action carrying its values
- **text:** Add the shared selection and paste round trip
- **platform:** Add selection and paste for macOS and Windows
- **snippets:** Add the record store for snippets and quicklinks
- **snippets:** Add snippets and quicklinks
- **windows:** Expose integrity level and handoff for driven tests
- **window-management:** Add the M4 extension and shared geometry
- **windows:** Implement the window manager
- **macos:** Manage windows through the accessibility api
- **window-management:** Add sizing commands and size cycling
- **config:** Load config.json and back preferences, enable, hotkey, aliases
- **config:** Watch the file and apply edits live
- **records:** Store snippets and quicklinks as text files
- **hotkeys:** Bind commands to global hotkeys from the config
- **config:** Add hyperkey config and a configurable hyper set
- **hyperkey:** Remap a key to the hyper modifier on Windows
- Start and live-reload the hyperkey from the config
- **snippets:** Add a keyword to snippets with word-boundary matching
- **snippets:** Expand keywords via a Windows key monitor
- **config:** Add a hyperkey tap key
- **hyperkey:** Send a key on a solitary tap, hyper on a hold
- **hyperkey:** Implement the macOS hyperkey
- **snippets:** Implement the macOS key monitor
- **ui:** Keep list selection off the pointer
- **clipboard:** Paste an entry into the active app
- Rewrite user-facing text to interface-copy spec
- **ai:** Add AI commands with BYOK providers
- **text:** Read the Windows selection through UI Automation
- **search:** Tint built-in icons per extension
- **quicklinks:** Show each site's favicon
- **search:** Head empty query with suggestions
- **ui:** Refine launcher presentation on shared tokens

### Fixes

- **launcher:** Resolve target display from the cursor
- **launcher:** Correct windows-sys imports and casts
- **launcher:** Keep tool window style across shows
- **launcher:** Position in native coordinate space
- **launcher:** Stop refocusing window after show
- **launcher:** Guard double dismiss and probe only shows
- **frontend:** Use fixed window height and a thin scrollbar
- **frontend:** Build root search on the bits-ui command primitive
- **frontend:** Even rows, footer bar, and a solid surface
- **frontend:** Run the root search on mount
- **frontend:** Let the keyboard keep the selection
- **frontend:** Show that a slow command is working
- **clipboard:** Make exclusion work against a real password manager
- **frontend:** Unify the two lists and settle focus ownership
- **clipboard:** Refuse a copy too large for the whole budget
- **clipboard:** Wait for the copying application on windows
- **macos:** Ask the application element for the selection first
- **spike:** Measure whether the target read the clipboard
- **text:** Measure the paste settle and stop guessing at the copy
- **text:** Dismiss the launcher before sending any keystroke
- **macos:** Send keystrokes from the main thread
- **macos:** Run window operations on the main thread
- **macos:** Run window operations on the main thread
- **ui:** Put the footer on the bottom and focus the form
- **windows:** Detect an elevated target by integrity, not an open probe
- **windows:** Relax the foreground lock when restoring focus
- **platform:** Drop needless return in the window-manager factory
- **text:** Re-assert the foreground in expand so the paste wins the restore
- **snippets:** Ignore modifier keys in the monitor
- **text:** Let the paste land before restoring the clipboard on expand
- **hyperkey:** Reclaim the remap a killed run left behind
- **ui:** Keep a failure visible past the hide
- **protocol:** Send the view owner with the tree
- **clipboard:** Recognise a re-encoded image as our own write
- Close macOS verification gaps
- **text:** Give the launcher insert the paste grace
- **clipboard:** Wait for a busy clipboard on Windows
- **launcher:** Drop renders of an abandoned run
- **text:** Ask about the application behind the launcher
- **macos:** Build against the new DirectSelection shape
- **quicklinks:** Sweep favicons from the records file

### Refactoring

- **clipboard:** Read the clipboard through clipboard-rs
- **macos:** Share the accessibility trust check

### Documentation

- Document command hotkeys in the example config
- **roadmap:** Record M5 shipped and verified on Windows
- Use the real clipboard preference keys in the example config
- **hotkeys:** Use an app name Windows really shows
- Release workflow proposal
- Document versioning and release procedure

### Maintenance

- Init repo with tauri and sveltekit scaffold
- Replace sveltekit with vite, bits-ui and tailwind
- Build and test on macos and windows
- **launcher:** Log frontend mounts to catch webview reloads
- Add windows crate for shell interop
- **clipboard:** Match text formats case-insensitively
- **clipboard:** Add the windows clipboard spike
- Roadmap small restructure
- **deps:** Add enigo, minijinja, and AX bindings with a spike
- **spike:** Name AX errors and probe the per-app element
- **spike:** Narrow the restore grid and repeat each delay
- **macos:** Drive the paste path against a real application
- **macos:** Cover the image clipboard branch in the harness
- **windows:** Add a driven walkthrough for selection and paste
- **windows:** Make the walkthrough drive a live desktop
- **windows:** Drop the foreground-rights crutch from the walkthrough
- **windows:** Add a driven walkthrough for window management
- **macos:** Walk the window manager through a real window
- **macos:** Check placement on every display
- **windows:** Cover the new sizes in the walkthrough
- Format the tree with rustfmt
- **macos:** Add the M5 key walkthrough harness
- **macos:** Give the walkthrough a window of its own
- **clipboard:** Apply rustfmt
- **ai:** Stop the two abandonment tests racing
- **windows:** Probe what applications expose to UI Automation
- **macos:** Cover the selection paths the trait change touched
- Git hooks
- Add browser fixtures and Playwright regressions
- Read the app version from Cargo.toml only
- Add release task with git-cliff and cargo-release
- Build installers and publish release on tag
