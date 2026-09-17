# Changelog

All notable changes to Dango. Generated from commit history at release time.

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
