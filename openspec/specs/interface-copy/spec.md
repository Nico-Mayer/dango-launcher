# interface-copy Specification

## Purpose

The writing standard for every piece of text Dango shows the user: result
titles, action labels, form labels, placeholders, empty states, status lines,
confirmations, failure messages, and the tray menu. It exists so that text
written for one feature reads like text written for any other, and so that a
failure always tells the user something they can act on.

## Requirements

### Requirement: Interface text is written for the reader

Every string Dango shows SHALL be written for the person reading it, not for
the developer who wrote it. Text SHALL use plain words, second person where a
person is addressed, and active voice. A sentence shown to the user SHALL be at
most 20 words. Text SHALL NOT contain an identifier, a symbol name, an error
code, a stack of quoted internals, or a hint that only makes sense with the
source open.

#### Scenario: A message is plain language

- **WHEN** any message, label, or state text is shown
- **THEN** it contains no internal identifier, type name, or numeric error code
- **AND** each sentence in it has 20 words or fewer

#### Scenario: A message addresses the user directly

- **WHEN** a message tells the user what to do
- **THEN** it uses the imperative or "you", not "the user"

### Requirement: Casing is sentence case, except for names

Text SHALL use sentence case: the first word capitalised, the rest lower case
except for proper nouns and product names. This applies to action labels, form
labels, placeholders, empty states, status lines, footer labels, tray items,
and messages.

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

- **WHEN** a failure, empty state, status line, or tray notice is shown
- **THEN** it starts with a capital letter and is otherwise lower case except
  for names

### Requirement: Action labels are a verb and its object

An action label SHALL start with an imperative verb and SHALL be at most four
words. It SHALL name what happens to the selected thing, so that it reads
correctly in the footer next to the Enter key. The verb SHALL match the effect:
"Delete" when the thing is gone for good, "Remove" only when it can come back,
"Open" for launching an application or a URL, "Paste" for putting text into the
previous application, "Copy" for putting it on the clipboard.

#### Scenario: A permanent action says delete

- **WHEN** the action panel offers to permanently discard a snippet, a
  quicklink, or a clipboard history entry
- **THEN** its label starts with "Delete"

#### Scenario: Launching an application says open

- **WHEN** an application result is selected in root search
- **THEN** its primary action is labelled "Open"
- **AND** the footer reads "Open" next to the Enter key

#### Scenario: A reveal action names the platform's file manager on macOS

- **WHEN** an application result's action panel is open on macOS
- **THEN** the reveal action is labelled "Show in Finder"

#### Scenario: A reveal action names the platform's file manager on Windows

- **WHEN** an application result's action panel is open on Windows
- **THEN** the reveal action is labelled "Show in File Explorer"

#### Scenario: A label fits the footer

- **WHEN** any view's primary action is shown in the footer
- **THEN** the label is four words or fewer

### Requirement: A failure says what failed and what to do

A failure shown in the banner SHALL be one or two complete sentences. The first
SHALL say what did not happen, in terms of the user's action. The second, when
there is something the user can do, SHALL say what. A failure message SHALL NOT
blame the user, SHALL NOT be a raw error from a library or the operating system,
and SHALL NOT end with a bare parenthesised code. Detail useful only for
debugging SHALL go to the log, not to the banner.

A failure caused by a missing permission SHALL name the permission and say how
to grant it. A failure caused by a thing that no longer exists SHALL say so and
let the user carry on.

#### Scenario: A command is no longer available

- **WHEN** the user runs a command whose extension has been disabled since the
  results were shown
- **THEN** the banner reads a sentence saying the command is no longer
  available and that the list will update on the next search

#### Scenario: A window cannot be moved on Windows

- **WHEN** a window-management command targets a window belonging to an
  elevated program on Windows
- **THEN** the banner says Dango cannot move windows of programs running as
  administrator
- **AND** the banner contains no window handle or Win32 error code

#### Scenario: A window cannot be moved on macOS

- **WHEN** a window-management command fails on macOS because the application
  does not expose its windows
- **THEN** the banner says that application does not let Dango move its
  windows
- **AND** the banner contains no `AXError` code

#### Scenario: The Accessibility permission is missing on macOS

- **WHEN** a paste, expansion, or window command fails on macOS for lack of the
  Accessibility permission
- **THEN** the banner names the Accessibility permission
- **AND** says it is granted in System Settings under Privacy and Security

#### Scenario: A system command is refused

- **WHEN** lock, sleep, or empty trash is refused by the operating system
- **THEN** the banner says the system did not carry out that action
- **AND** the operating system's own error text is written to the log, not shown

#### Scenario: An application has gone

- **WHEN** the user opens an application that has been uninstalled since it was
  indexed
- **THEN** the banner says the application is no longer installed and has been
  removed from results

#### Scenario: Form validation fails

- **WHEN** the user saves a snippet or quicklink form that is missing a name,
  has an invalid URL, or reuses a keyword
- **THEN** the banner names the field and what it needs, such as "Enter a
  name" or "Another snippet already uses that keyword"

#### Scenario: A template has a problem

- **WHEN** the user edits a snippet template into a state the template engine
  cannot parse
- **THEN** the form shows a line saying the template cannot be read, followed
  by the engine's description of where
- **AND** the form still accepts further edits

### Requirement: Empty states name the thing and the next step

An empty state SHALL have a title that says what is absent, and MAY have one
sentence saying how to fill it. It SHALL NOT be a generic phrase such as
"Nothing here" or "That did not work". An empty state shown because of an
error SHALL follow the failure requirement above.

#### Scenario: Root search finds nothing

- **WHEN** the user types a query with no matching result
- **THEN** the list shows "No results for" followed by the query

#### Scenario: A pushed list is empty

- **WHEN** a command's list view has no items and declared an empty state
- **THEN** the declared title and description are shown as given

#### Scenario: A pushed list is empty without a declared state

- **WHEN** a command's list view has no items and declared no empty state
- **THEN** the list shows "Nothing to show"

#### Scenario: A pushed list is filtered to nothing

- **WHEN** the user types into a pushed list and no item matches
- **THEN** the list shows "No results for" followed by the query

#### Scenario: A history is empty on first use

- **WHEN** clipboard history is opened before anything has been copied
- **THEN** the title says nothing has been copied yet
- **AND** the description says what will appear here, in one sentence

### Requirement: Status lines say what is happening

A status line shown while Dango works SHALL say what is being waited for. The
form's template inspection SHALL say what the template will ask for, or that it
will ask for nothing, in words the user would use.

#### Scenario: A command is running

- **WHEN** a command has been invoked from root search and has neither
  finished nor shown a view yet
- **THEN** the root view shows a status line reading "Running" followed by the
  command's title

#### Scenario: A pushed list is loading

- **WHEN** a list view is marked loading and has no items yet
- **THEN** the list shows "Loading…"

#### Scenario: A template asks for input

- **WHEN** a snippet template references two names that are not reserved
- **THEN** the form reads "Will ask for" followed by the two names

#### Scenario: A template asks for nothing

- **WHEN** a snippet template references only reserved names, or none
- **THEN** the form reads "Nothing to fill in"

### Requirement: Confirmations name the loss and make backing out easy

A confirmation before an irreversible action SHALL state what will be lost, in
a count and a noun, SHALL say it cannot be undone, and SHALL offer a cancel
action first so that Enter backs out. The confirming action SHALL repeat the
verb and the count.

#### Scenario: Emptying the trash

- **WHEN** the user invokes empty trash with items in the trash
- **THEN** the detail view asks whether to permanently delete that many items
  and says it cannot be undone
- **AND** the first action is "Cancel" and the second is "Delete" followed by
  the count and noun

### Requirement: Tray notices say what is wrong and where to look

A tray notice about a startup problem SHALL name the thing that failed and,
when the detail is in the log, SHALL name the log file. A notice about the
launcher hotkey SHALL name the chord and say what to change. The config status
line SHALL say whether the config file loaded.

#### Scenario: Hotkeys conflict

- **WHEN** one or more command hotkeys could not be registered at startup
- **THEN** a disabled tray item reads that some hotkeys are already in use and
  names `dango.log`

#### Scenario: The launcher hotkey is taken on macOS

- **WHEN** Option+Space is already registered by another application
- **THEN** a tray item says Option+Space is in use by another app and points to
  the launcher hotkey setting in the config file

#### Scenario: The launcher hotkey is taken on Windows

- **WHEN** Alt+Space is already registered by another application
- **THEN** a tray item says Alt+Space is in use by another app and points to
  the launcher hotkey setting in the config file

#### Scenario: The config file has an error

- **WHEN** the config file did not parse
- **THEN** the tray status line reads "Config file has an error" and names
  `dango.log`

#### Scenario: The config file loaded

- **WHEN** the config file parsed
- **THEN** the tray status line reads "Config file loaded"

#### Scenario: Tray menu items

- **WHEN** the tray menu is open
- **THEN** the actionable items read "Show or hide Dango" and "Quit Dango"

### Requirement: Terminology is consistent

One thing SHALL have one name everywhere it appears. The established names are:
"snippet", "quicklink", "clipboard history", "entry" for one item of the
history, "keyword" for a snippet's expansion trigger, "hotkey" for a global
chord, "config file" for `config.json`, "the launcher" for Dango's window, and
"the previous app" for the application the user was in before summoning
Dango. A platform component SHALL be called what the platform calls it.

#### Scenario: A history item is an entry

- **WHEN** clipboard history text refers to one item
- **THEN** it says "entry", not "item", "record", or "clip"

#### Scenario: The application behind the launcher

- **WHEN** a message refers to the application the user came from
- **THEN** it says "the previous app", not "active app", "target", or
  "foreground window"

### Requirement: New interface text is held to this standard

Every change that adds or edits text the user can see SHALL list the new or
changed strings in its design, and each SHALL meet the requirements in this
specification. A string that appears in a scenario of another spec SHALL be
quoted there in its final form.

#### Scenario: A change adds a message

- **WHEN** a change's design introduces a new failure, empty state, label, or
  notice
- **THEN** the design quotes the string
- **AND** the string meets the casing, length, and shape rules above
