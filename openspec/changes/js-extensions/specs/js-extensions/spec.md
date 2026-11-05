# Spec Delta

## Purpose

Lets a user add commands to Dango by dropping a folder of JavaScript into the
config directory, without changing Dango itself: how such extensions are found,
run, reloaded, and kept from affecting each other.

## ADDED Requirements

### Requirement: Extensions load from the extensions folder

Dango SHALL load every folder in `extensions/` in the config directory,
honouring `$DANGO_CONFIG_DIR`, that holds a `manifest.json` and a `main.js`.
The folder name SHALL be the extension's id. Extensions SHALL load at startup.
A missing `extensions` folder SHALL mean no such extensions and SHALL NOT be
reported as a problem. A folder without both files SHALL be skipped and
reported.

#### Scenario: An extension folder is loaded

- **WHEN** `extensions/gh-prs/` holds a valid `manifest.json` and `main.js` and
  Dango starts
- **THEN** the extension `gh-prs` is loaded and its commands appear in root
  search

#### Scenario: No extensions folder

- **WHEN** the config directory has no `extensions` folder
- **THEN** Dango starts normally and reports no problem

#### Scenario: The override moves the folder

- **WHEN** `$DANGO_CONFIG_DIR` is set
- **THEN** extensions are loaded from `extensions/` in that directory

#### Scenario: A folder without main.js

- **WHEN** an extension folder holds `manifest.json` but no `main.js`
- **THEN** that extension is not loaded
- **AND** the tray shows "Extension <id> didn't load, see dango.log"

### Requirement: The manifest is a JSON file

`manifest.json` SHALL carry the manifest that `extension-model` defines:
`manifestVersion`, `id`, `name`, an optional `icon`, its `commands`, and an
optional `rootItems` with an optional `refreshMinutes`. Keys SHALL be
camelCase, as in the view protocol. A manifest that is not valid JSON, whose
`id` differs from the folder name, or that declares `services` SHALL NOT load
and SHALL be reported.

#### Scenario: A valid manifest

- **WHEN** a manifest declares version 1, id `gh-prs`, name "GitHub PRs", and
  one view command
- **THEN** the extension loads with that name and command

#### Scenario: The id does not match the folder

- **WHEN** the folder `gh-prs` holds a manifest with id `github`
- **THEN** the extension is not loaded
- **AND** the problem is reported in the tray and the log

#### Scenario: A manifest declares services

- **WHEN** a manifest declares `services`
- **THEN** the extension is not loaded and the problem is reported

### Requirement: Searching runs no extension code

An extension's declared commands SHALL appear in root search, be matched by
title, keywords, and alias, and be bound to hotkeys from the manifest and
config file alone. Dango SHALL NOT run an extension's JavaScript to search,
list, or bind its commands. The JavaScript SHALL run first when a command is
invoked or root items are built.

#### Scenario: A command is found before its code runs

- **WHEN** an extension's `main.js` writes a file as soon as it runs, and the
  user types part of one of its command titles
- **THEN** the command appears in root search
- **AND** the file has not been written

### Requirement: Each declared command runs exported code

`main.js` SHALL export one function per declared command, under the command's
id. Invoking the command SHALL call that function. A declared command without
an exported function SHALL still appear in root search, and invoking it SHALL
show the failure ticker "This command can't run. Its extension has no code
for it." while the extension's other commands keep working.

#### Scenario: Invoking calls the export

- **WHEN** the user runs a command whose id is `my-prs`
- **THEN** the function `main.js` exports as `my-prs` is called

#### Scenario: A command without code

- **WHEN** the user runs a declared command that `main.js` does not export
- **THEN** the launcher stays open with the failure ticker "This command can't
  run. Its extension has no code for it."

### Requirement: A view command shows the views it emits

A `view` command SHALL be able to emit any number of view trees while it runs.
Each one SHALL replace the last, as the view protocol's full-tree replace
defines. A command SHALL be able to emit a loading view first and its content
later.

#### Scenario: Loading, then content

- **WHEN** a view command emits a list marked loading and, a moment later, a
  list of five items
- **THEN** the launcher shows the loading marks and then the five items

### Requirement: A no-view command reports its outcome

A `no-view` command SHALL succeed when its function returns, with the message
it returns, if any, as the success ticker. It SHALL fail when it reports a
failure message, and that message SHALL be shown as given. Both then follow
the view protocol's no-view outcome rules.

#### Scenario: A command succeeds with a message

- **WHEN** a no-view command returns the message "VPN connected"
- **THEN** the launcher closes if it is open, and the outcome is a success
  with that message

#### Scenario: A command reports a failure

- **WHEN** a no-view command reports the failure "VPN is not installed"
- **THEN** an open launcher stays open and shows "VPN is not installed" in the
  failure ticker

### Requirement: An uncaught error is shown without internals

When an extension's code throws an error it does not catch, or a promise it
returns is rejected, the failure ticker SHALL read "This command stopped
because of an error, see dango.log". The error and its stack SHALL go to the
log. The launcher SHALL stay usable.

#### Scenario: A command throws

- **WHEN** a command's function throws a `TypeError`
- **THEN** the failure ticker reads "This command stopped because of an error,
  see dango.log"
- **AND** the log holds the error message and the extension's id

### Requirement: Actions run extension code

An action on an item or view of an extension SHALL run the extension's code.
That code SHALL be able to push a new view, replace the current one, show a
success ticker, report a failure, go back one level, or finish and close the
launcher. A form submit SHALL deliver the field values to that code.

#### Scenario: An action pushes a view

- **WHEN** the user presses Enter on an item whose primary action pushes a
  detail view
- **THEN** the detail view is shown above the list
- **AND** Escape returns to the list

#### Scenario: An action replaces the view

- **WHEN** an action removes an item and emits the list without it
- **THEN** the list is shown without that item and the launcher stays open

#### Scenario: A form submit reaches the code

- **WHEN** the user submits a form with the field "title" set to "Fix login"
- **THEN** the extension's submit code receives "Fix login" for "title"

### Requirement: Root items are built once and cached

An extension whose manifest declares `rootItems` SHALL have its root items
built when it loads, when it is enabled, when it reloads, and, if
`refreshMinutes` is set, every that many minutes while enabled. Dango SHALL
keep the last rows and match them against each query itself. Building SHALL
NOT delay root search: until rows exist, the extension adds none.

#### Scenario: Root items appear in search

- **WHEN** an extension's root items are three repositories and the user types
  part of one name
- **THEN** that repository appears in root search, ranked with other results

#### Scenario: Typing does not run the extension

- **WHEN** root items have been built and the user types ten characters
- **THEN** the extension's code is not run for any of them

#### Scenario: Rows refresh on the interval

- **WHEN** `refreshMinutes` is 30 and the extension's data changed
- **THEN** within 30 minutes root search shows the new rows

#### Scenario: A failed refresh keeps the last rows

- **WHEN** a refresh throws an error after an earlier build succeeded
- **THEN** root search keeps showing the earlier rows
- **AND** the error is written to the log

### Requirement: A root item carries a title and actions

A root item SHALL have an id and a title and MAY have a subtitle, an icon,
keywords, and actions. Its first action SHALL run on Enter, and the action
panel SHALL reach the rest. Running an action of a root item SHALL run the
extension's code even when nothing of that extension is running.

#### Scenario: Enter on a root item

- **WHEN** the user selects an extension's root item and presses Enter
- **THEN** that item's first action runs

### Requirement: Extensions can run programs

An extension SHALL be able to run a program with arguments, and receive its
exit code, standard output, and standard error. No window SHALL appear for
the program. Programs SHALL be found on the same search path Dango uses for
agent CLIs.

#### Scenario: Running a CLI on Windows

- **WHEN** an extension on Windows runs `gh` with the arguments `pr list`
- **THEN** it receives `gh`'s output and exit code
- **AND** no console window appears

#### Scenario: Running a CLI on macOS

- **WHEN** Dango on macOS was started from Finder and an extension runs a
  program installed by Homebrew
- **THEN** the program is found and its output is returned

### Requirement: Extensions can use files, network, and timers

An extension SHALL be able to read, write, list, and delete files anywhere the
user can, make HTTP requests and read their responses, and wait with timers.
Dango SHALL NOT ask for permission for any of these.

#### Scenario: Reading a file

- **WHEN** an extension reads a file in the user's home folder
- **THEN** it receives the file's contents

#### Scenario: Calling an API

- **WHEN** an extension requests a JSON document over HTTPS
- **THEN** it receives the status and the body

### Requirement: Extensions can open, copy, and paste

An extension SHALL be able to open a URL or file with its default application,
put text on the clipboard, and paste text into the previous app, through the
same paths built-ins use.

#### Scenario: Opening a URL

- **WHEN** an extension's action opens a URL
- **THEN** the URL opens in the default browser and the launcher closes

### Requirement: Each extension has its own storage

An extension SHALL be able to store, read, and delete values under keys of its
choosing. Stored values SHALL survive a restart and a reload. An extension
SHALL NOT read another extension's values through this storage. The values are
machine-local.

#### Scenario: A value survives a restart

- **WHEN** an extension stores a value and Dango restarts
- **THEN** the extension reads the same value

#### Scenario: Storage is per extension

- **WHEN** two extensions store a value under the same key
- **THEN** each reads back its own value

### Requirement: Preferences come from the config file

An extension SHALL receive as its preferences every key in its
`extensions.<id>` table of the config file other than `enabled` and
`commands`. A change to them SHALL apply on reload of the config file, without
a restart: code that runs afterwards sees the new values.

#### Scenario: A preference reaches the extension

- **WHEN** the config file sets `user = "nico"` in `[extensions.gh-prs]`
- **THEN** the extension's code reads `user` as "nico"

### Requirement: Extension code runs only while it has work

An extension's code SHALL run only while one of its commands runs, one of its
views is shown, or its root items are being built. When that work ends, its
state, pending timers, and unfinished requests SHALL be dropped. A command
whose launcher view is left SHALL be stopped.

#### Scenario: Leaving a view stops its code

- **WHEN** an extension's view is open with a timer that would push a new view
  in a minute, and the user presses Escape
- **THEN** no view from that timer appears

#### Scenario: Idle extensions hold nothing

- **WHEN** an extension's command finished and its view was closed
- **THEN** its pending timers do not fire

### Requirement: Runaway code does not freeze the launcher

An extension that loops forever or blocks SHALL NOT make the launcher
unresponsive. Leaving its view, hiding the launcher, or running another
command SHALL stop it. Code that exceeds its memory limit SHALL be stopped
with the failure ticker "This command used too much memory, so Dango stopped
it."

#### Scenario: An endless loop

- **WHEN** a command loops forever and the user presses Escape
- **THEN** the launcher responds and the loop is stopped

#### Scenario: Too much memory

- **WHEN** a command keeps allocating memory past its limit
- **THEN** it is stopped
- **AND** the failure ticker reads "This command used too much memory, so
  Dango stopped it."

### Requirement: Editing an extension reloads it

Dango SHALL watch the `extensions` folder. A change to a file in an extension's
folder SHALL reload that extension without a restart. A new folder SHALL load,
and a removed one SHALL unregister its commands, root items, and hotkeys. A
view of the extension that is open during a reload SHALL keep working until
the user leaves it.

#### Scenario: Editing main.js

- **WHEN** the user saves a changed `main.js` while Dango runs
- **THEN** the next run of the command uses the new code

#### Scenario: Removing a folder

- **WHEN** the user deletes an extension's folder
- **THEN** its commands leave root search and its hotkeys are released

#### Scenario: A reload that breaks the extension

- **WHEN** the user saves a manifest that is not valid JSON
- **THEN** the extension's commands leave root search
- **AND** the tray shows "Extension <id> didn't load, see dango.log"
- **AND** every other extension keeps working

### Requirement: Dango writes type definitions for authors

When the `extensions` folder exists, Dango SHALL write `dango.d.ts` there at
startup, describing the manifest, the view protocol, and every capability
available to extension code, matching the running version. Dango SHALL NOT
create the `extensions` folder for this, and SHALL NOT write any other file in
it.

#### Scenario: The types file is written

- **WHEN** the `extensions` folder exists and Dango starts
- **THEN** `extensions/dango.d.ts` exists and describes the view protocol at
  version 3

#### Scenario: No folder, no file

- **WHEN** there is no `extensions` folder and Dango starts
- **THEN** Dango creates neither the folder nor the file
