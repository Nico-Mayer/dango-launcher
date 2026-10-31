## MODIFIED Requirements

### Requirement: Casing is sentence case, except for names

Text SHALL use sentence case: the first word capitalised, the rest lower case
except for proper nouns and product names. This applies to action labels, form
labels, placeholders, empty states, toasts, ActionBar labels, section headings,
tags, tray items, and messages.

Command titles in root search SHALL be Title Case, because they are shown in
one list with application names and are searched as names. Extension names
SHALL be Title Case for the same reason. A command's title is the only Title
Case text in the interface.

Platform components keep their own names: Finder, File Explorer, Trash,
Recycle Bin, Accessibility, Cmd, Ctrl, Option, Alt, Enter, Escape.

#### Scenario: Action labels are sentence case

- **WHEN** an action panel is open for any result
- **THEN** every action label reads in sentence case, such as "Show in Finder"
  or "Copy path"

#### Scenario: Command titles are Title Case

- **WHEN** root search lists a built-in command
- **THEN** its title is Title Case, such as "Empty Trash" or "Clipboard History"

#### Scenario: A message is sentence case

- **WHEN** a failure, empty state, toast, or tray notice is shown
- **THEN** it starts with a capital letter and is otherwise lower case except
  for names

### Requirement: Action labels are a verb and its object

An action label SHALL start with an imperative verb and SHALL be at most four
words. It SHALL name what happens to the selected thing, so that it reads
correctly in the ActionBar next to the Enter key. The verb SHALL match the
effect: "Delete" when the thing is gone for good, "Remove" only when it can
come back, "Open" for launching an application or a URL, "Paste" for putting
text into the previous application, "Copy" for putting it on the clipboard.

#### Scenario: A permanent action says delete

- **WHEN** the action panel offers to permanently discard a snippet, a
  quicklink, or a clipboard history entry
- **THEN** its label starts with "Delete"

#### Scenario: Launching an application says open

- **WHEN** an application result is selected in root search
- **THEN** its primary action is labelled "Open"
- **AND** the ActionBar reads "Open" next to the Enter key

#### Scenario: A reveal action names the platform's file manager on macOS

- **WHEN** an application result's action panel is open on macOS
- **THEN** the reveal action is labelled "Show in Finder"

#### Scenario: A reveal action names the platform's file manager on Windows

- **WHEN** an application result's action panel is open on Windows
- **THEN** the reveal action is labelled "Show in File Explorer"

#### Scenario: A label fits the footer

- **WHEN** any view's primary action is shown in the ActionBar
- **THEN** the label is four words or fewer

### Requirement: A failure says what failed and what to do

A failure SHALL be shown as a failure toast of one sentence that fits the toast
without being cut off at the launcher's width. When the user can do something
about it, the sentence SHALL say what. Otherwise it SHALL say what did not
happen, in terms of the user's action. A failure message SHALL NOT blame the
user, SHALL NOT be a raw error from a library or the operating system, and
SHALL NOT end with a bare parenthesised code. Detail useful only for debugging
SHALL go to the log, not to the toast.

A failure caused by a missing permission SHALL name the permission and where
to grant it. A failure caused by a thing that no longer exists SHALL say so.

#### Scenario: A command is no longer available

- **WHEN** the user runs a command whose extension has been disabled since the
  results were shown
- **THEN** the toast reads "That command is no longer available"

#### Scenario: A window cannot be moved on Windows

- **WHEN** a window-management command targets a window belonging to an
  elevated program on Windows
- **THEN** the toast says Dango can't move administrator windows
- **AND** the toast contains no window handle or Win32 error code

#### Scenario: A window cannot be moved on macOS

- **WHEN** a window-management command fails on macOS because the application
  does not expose its windows
- **THEN** the toast says that app doesn't let Dango move its windows
- **AND** the toast contains no `AXError` code

#### Scenario: The Accessibility permission is missing on macOS

- **WHEN** a paste, expansion, or window command fails on macOS for lack of the
  Accessibility permission
- **THEN** the toast reads "Allow Accessibility in System Settings"

#### Scenario: A system command is refused

- **WHEN** lock, sleep, or empty trash is refused by the operating system
- **THEN** the toast says the system didn't do that action
- **AND** the operating system's own error text is written to the log, not shown

#### Scenario: An application has gone

- **WHEN** the user opens an application that has been uninstalled since it was
  indexed
- **THEN** the toast reads "That app is no longer installed"
- **AND** the application is removed from results

#### Scenario: Form validation fails

- **WHEN** the user saves a snippet or quicklink form that is missing a name,
  has an invalid URL, or reuses a keyword
- **THEN** the toast names the field and what it needs, such as "Enter a name"
  or "Another snippet already uses that keyword"

#### Scenario: A template has a problem

- **WHEN** the user edits a snippet template into a state the template engine
  cannot parse
- **THEN** the form shows, under the template field, a line saying the template
  cannot be read, followed by the engine's description of where
- **AND** the form still accepts further edits

### Requirement: Status lines say what is happening

A busy toast shown while Dango works SHALL say what is being waited for. The
form's template inspection SHALL say what the template will ask for, or that it
will ask for nothing, in words the user would use.

#### Scenario: A command is running

- **WHEN** a command has been invoked from root search and has neither
  finished nor shown a view yet
- **THEN** a busy toast reads "Running" followed by the command's title

#### Scenario: A pushed list is loading

- **WHEN** a list view is marked loading and has no items yet
- **THEN** a busy toast reads "Loading…"

#### Scenario: A template asks for input

- **WHEN** a snippet template references two names that are not reserved
- **THEN** the form reads "Will ask for" followed by the two names

#### Scenario: A template asks for nothing

- **WHEN** a snippet template references only reserved names, or none
- **THEN** the form reads "Nothing to fill in"
