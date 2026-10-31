# view-protocol Specification

## Purpose

The versioned declarative contract describing what a command puts on screen and
which actions it offers, so that the frontend renders any command, built-in or
otherwise, without knowing anything about it.

## Requirements

### Requirement: Versioned protocol

The view protocol SHALL carry a `protocolVersion`. Dango SHALL reject a view
tree whose version it does not support and SHALL show an error rather than
rendering partially. The check SHALL happen before the tree reaches the
renderer, so every renderer shows the same error for the same tree.

#### Scenario: Unsupported protocol version

- **WHEN** a command emits a view tree with an unsupported `protocolVersion`
- **THEN** an error is shown in place of the view
- **AND** the user can dismiss it and return to root

### Requirement: View kinds

The protocol SHALL be at version 2 and SHALL support four view kinds: a list, a
grid, a detail view rendering markdown, and a form. Any view kind SHALL be able
to carry an action panel. A view tree at version 1 SHALL be rejected as an
unsupported version.

#### Scenario: List renders items

- **WHEN** a command emits a list view containing items
- **THEN** each item's title, and its subtitle, icon, and accessories when
  present, are rendered in order

#### Scenario: Grid renders tiles

- **WHEN** a command emits a grid view containing items
- **THEN** each item is rendered as a tile with its content and title, in order,
  in the declared number of columns

#### Scenario: Detail renders markdown

- **WHEN** a command emits a detail view
- **THEN** its markdown content is rendered

#### Scenario: Form collects input

- **WHEN** a command emits a form view and the user submits it
- **THEN** the entered values are delivered to the command

#### Scenario: A version 1 tree

- **WHEN** a command emits a view tree at protocol version 1
- **THEN** an error is shown in place of the view

### Requirement: Full-tree replace

A command SHALL communicate a change by emitting a complete new view tree. The
protocol SHALL NOT define partial updates, patches, or diffs. Dango SHALL
identify list items by their identifier so that replacing a tree does not lose
selection or cause visible flicker.

#### Scenario: Selection survives a tree replacement

- **WHEN** the user has selected the third item and the command emits a new tree containing that same item identifier
- **THEN** the item remains selected

#### Scenario: Streaming content replaces the tree repeatedly

- **WHEN** a command emits thirty view trees over one second
- **THEN** each renders without flicker and without dropped frames

### Requirement: Action panel

Any view SHALL be able to declare a list of actions, the first of which is the
primary action. The primary action SHALL be invocable with Enter and the panel
SHALL be openable to reach the remaining actions. Each action MAY declare its
own keyboard shortcut, a section, and a style of normal or danger.

The panel SHALL list the actions in order, grouping those that declare a
section under that section's heading, and SHALL show danger actions in the
danger role after all others. The panel SHALL have its own filter input: typing
while it is open SHALL narrow the actions by title, select the first match, and
SHALL NOT change the query below. When nothing matches, the panel SHALL say so.

The panel SHALL open on Ctrl+K on Windows and on Cmd+K or Ctrl+K on macOS, for
the selected row, and that chord SHALL do nothing else. Ctrl combined with J, K,
N, or P SHALL NOT move the selection in any list; only the arrow keys move it.

Wherever the launcher shows a chord, it SHALL name the modifier the platform
uses: Cmd on macOS and Ctrl on Windows. The ActionBar SHALL name the action
Enter runs: a list's or grid's selected item's primary action, or the view's
own primary action.

#### Scenario: Primary action runs on Enter

- **WHEN** a view declares actions and the user presses Enter
- **THEN** the first action is invoked

#### Scenario: Secondary action is reachable

- **WHEN** the user opens the action panel and chooses a secondary action
- **THEN** that action is invoked

#### Scenario: View declares no actions

- **WHEN** a view declares no actions and the user presses Enter
- **THEN** nothing is invoked and the view is unchanged

#### Scenario: Opening the panel on Windows

- **WHEN** a row with more than one action is selected and the user presses
  Ctrl+K
- **THEN** the action panel opens listing that row's actions
- **AND** the selected row is the same row as before the press

#### Scenario: Opening the panel on macOS

- **WHEN** a row with more than one action is selected and the user presses
  Cmd+K or Ctrl+K
- **THEN** the action panel opens listing that row's actions
- **AND** the selected row is the same row as before the press

#### Scenario: Ctrl with a letter does not move the selection

- **WHEN** a list is showing and the user presses Ctrl+J, Ctrl+N, or Ctrl+P
- **THEN** the selected row is unchanged
- **AND** the action panel does not open

#### Scenario: Sections and danger actions

- **WHEN** an item declares "Pin entry" and "Delete entry" in a section "Manage",
  with "Delete entry" in the danger style
- **THEN** the panel shows them under the heading "Manage", with "Delete entry"
  last and in the danger role

#### Scenario: Filtering the panel

- **WHEN** the panel is open and the user types "co"
- **THEN** only actions whose titles match "co" are listed, the first is
  selected, and the query below is unchanged

#### Scenario: Nothing matches in the panel

- **WHEN** the panel's filter matches no action
- **THEN** the panel shows "No matching actions" and Enter runs nothing

#### Scenario: The footer names the platform's chord on macOS

- **WHEN** the launcher is open on macOS
- **THEN** the ActionBar shows the action panel chord with the Cmd symbol
- **AND** an action's shortcut in the panel is shown with the Cmd symbol

#### Scenario: The footer names the platform's chord on Windows

- **WHEN** the launcher is open on Windows
- **THEN** the ActionBar shows the action panel chord as Ctrl+K
- **AND** an action's shortcut in the panel is shown with Ctrl

#### Scenario: The footer names the primary action of a list

- **WHEN** a list view is showing with a selected row that has actions
- **THEN** the ActionBar names that row's first action next to the Enter key

#### Scenario: The rule holds in root search and in a pushed list

- **WHEN** the user presses Ctrl+K in root search, or in a list view a command
  pushed
- **THEN** in both places the panel opens for the selected row and the selection
  does not move

### Requirement: Loading and empty states

A view SHALL be able to signal that it is loading and to declare what is shown
when it has no content.

Every list, grid, or detail view marked loading SHALL show the SearchBar's
loading bar and a busy toast reading "Loading…", whether or not it already
contains content. Entering the loading state SHALL NOT itself clear, replace,
dim into unreadability, or block interaction with content supplied by the
command. The loading marks SHALL reflect the declared loading state, not infer
completion from incoming text or elapsed time, and SHALL NOT claim a numerical
progress value.

An empty state SHALL have a title and MAY have an icon, a one-line description,
and actions. When an empty view is shown, Enter SHALL run its first action.

Busy state SHALL be exposed to assistive technology. Status announcements SHALL
describe state changes without announcing every streamed replacement.

#### Scenario: Loading is visible

- **WHEN** a command emits a view marked as loading
- **THEN** the loading bar and the busy toast are shown without clearing any
  content already displayed

#### Scenario: Empty state is shown

- **WHEN** a list or grid contains no items and is not loading
- **THEN** its declared empty state is shown

#### Scenario: Empty state carries an action

- **WHEN** an empty state declares the action "Search the web" and the user
  presses Enter
- **THEN** that action runs
- **AND** the ActionBar names it next to the Enter key

#### Scenario: Empty list is loading

- **WHEN** a command emits a loading list with no items
- **THEN** the busy toast reads "Loading…" and the completed empty state is not
  shown

#### Scenario: Populated list is refreshing

- **WHEN** a command emits a loading list with existing items
- **THEN** those items remain visible and selectable, and the loading marks are
  shown
- **AND** the existing item-identity rules continue to preserve selection

#### Scenario: Detail content is streaming

- **WHEN** a command repeatedly replaces a detail view marked loading
- **THEN** the supplied content remains visible alongside the loading marks
- **AND** the status is not announced again for each replacement

#### Scenario: Loading finishes

- **WHEN** the command emits a replacement with loading set to false
- **THEN** its content remains visible and the loading marks are removed
- **AND** assistive technology is no longer told that the view is busy

#### Scenario: Loading feedback is abandoned

- **WHEN** the user leaves a loading view or hides the launcher
- **THEN** its loading marks are removed
- **AND** a late update from an abandoned invocation does not restore them

#### Scenario: Reduced motion preserves loading feedback

- **WHEN** a list or detail view is loading with reduced motion enabled
- **THEN** the same static loading bar and "Loading…" toast are shown as
  without reduced motion

### Requirement: No-view commands report their outcome

A command declared as `no-view` SHALL NOT emit a view tree. It SHALL report
success or failure, and the launcher SHALL close on success while showing the
message in a failure toast on failure.

#### Scenario: No-view command succeeds

- **WHEN** a `no-view` command completes successfully
- **THEN** the launcher hides

#### Scenario: No-view command fails

- **WHEN** a `no-view` command fails
- **THEN** the launcher stays open and the failure is shown as a failure toast

### Requirement: View stack

Views SHALL form a stack. Invoking an action that opens a view SHALL push onto
the stack, Escape or the SearchBar's back control SHALL pop one level, and
Escape at the root SHALL hide the launcher. Hiding the launcher SHALL clear the
stack back to root. Each level SHALL keep its query, filter, and selection, so
going back restores them.

#### Scenario: Escape pops one level

- **WHEN** the user is two views deep and presses Escape
- **THEN** the previous view is shown with its prior state intact

#### Scenario: The back control pops one level

- **WHEN** the user is two views deep and clicks the back control
- **THEN** the previous view is shown with its prior state intact

#### Scenario: Escape at root hides the launcher

- **WHEN** the user is at the root view and presses Escape
- **THEN** the launcher hides

#### Scenario: Reopening starts at root

- **WHEN** the user is three views deep, hides the launcher, and reopens it
- **THEN** the root view is shown with an empty query

### Requirement: List and grid items can be grouped in sections

A list or grid SHALL be able to group its items in sections. A section has a
title and MAY have a subtitle. Items SHALL be shown in section order and, inside
a section, in their own order. A section with no items SHALL NOT be shown. When
the launcher filters, a section whose items are all filtered out SHALL be
hidden.

#### Scenario: Sections are shown in order

- **WHEN** a list declares a section "Today" with three items and a section
  "Yesterday" with one
- **THEN** "Today" and its items are shown above "Yesterday" and its item

#### Scenario: Filtering hides an empty section

- **WHEN** the launcher filters a sectioned list and no item of "Yesterday"
  matches
- **THEN** the "Yesterday" heading is not shown

#### Scenario: Selection crosses sections

- **WHEN** the last item of one section is selected and the user presses Arrow
  Down
- **THEN** the first item of the next section is selected

### Requirement: Items can carry accessories and keywords

A list item SHALL be able to declare up to three accessories, each one of text,
an icon, a tag with a role of accent, muted, or danger, or a key chord. An item
SHALL be able to declare keywords. When the launcher filters, a query SHALL
match an item by its keywords as well as its title, and the keywords SHALL NOT
be shown.

#### Scenario: Accessories are shown

- **WHEN** an item declares the text "2m" and a tag "Link" in the accent role
- **THEN** the row shows "2m" and the "Link" tag on its right

#### Scenario: A keyword matches

- **WHEN** an item titled "Terminal" declares the keyword "shell" and the user
  types "shell"
- **THEN** the item is shown

### Requirement: A list can show the selected item's detail

A list view SHALL be able to declare a detail pane. Each item MAY then carry a
detail, with markdown and metadata, which is shown beside the list while the
item is selected. Moving the selection SHALL show the newly selected item's
detail. An item without a detail SHALL leave the pane empty.

#### Scenario: Moving the selection updates the pane

- **WHEN** a list with a detail pane is shown and the user presses Arrow Down
- **THEN** the pane shows the detail of the newly selected item

### Requirement: A detail can carry metadata

A detail, standalone or in a list's pane, SHALL be able to declare metadata: an
ordered list of label and value pairs shown below its body.

#### Scenario: Metadata is shown

- **WHEN** a detail declares the pairs "Source" with "Terminal" and "Copied" with
  "Today, 14:31"
- **THEN** both pairs are shown below the body, in that order

### Requirement: Grid view

A grid view SHALL declare its number of columns, from 4 to 8, and an aspect
ratio for its tiles. Each item's content SHALL be an image, a colour, or a
single glyph, and the item SHALL have a title and MAY have a subtitle and
actions. The grid SHALL share the list's sections, selection, filtering,
empty state, and action rules.

#### Scenario: Columns are respected

- **WHEN** a grid declares five columns and holds twelve items
- **THEN** the items fill three rows of five, five, and two

#### Scenario: Columns out of range

- **WHEN** a grid declares two columns
- **THEN** the view is rejected and an error is shown in its place

### Requirement: A view can declare a filter dropdown

A list or grid view SHALL be able to declare a filter dropdown: a list of
options with titles and identifiers, and the selected option. The SearchBar
SHALL show the selected option's title. Choosing another option SHALL deliver
its identifier to the command, which MAY emit a new view tree. The query SHALL
be kept when the filter changes.

The dropdown SHALL open on a click. So the filter is reachable from the
keyboard, the action panel SHALL also list the options under a "Filter"
section, after the selected item's actions and before its danger actions, with
the selected option marked.

#### Scenario: Choosing a filter

- **WHEN** clipboard history declares the options "All types" and "Text only"
  and the user chooses "Text only"
- **THEN** the command receives the "Text only" identifier
- **AND** the SearchBar shows "Text only" and keeps the query

#### Scenario: Choosing a filter from the keyboard

- **WHEN** a view declares a filter dropdown, the user opens the action panel,
  and confirms "Text only" under "Filter"
- **THEN** the command receives the "Text only" identifier and the panel closes

### Requirement: Form field kinds

A form field SHALL be one of: a single-line text field, a password field, a
multi-line text area, a template, a checkbox, or a dropdown with options. A
form MAY place a separator between fields. Every field kind except the
checkbox SHALL show its label in the label column; a checkbox SHALL show its
label beside it. Tab and Shift+Tab SHALL move between fields in order.

#### Scenario: A dropdown field

- **WHEN** a form declares a dropdown field "Folder" with the options "Work" and
  "Personal" and the user chooses "Personal"
- **THEN** submitting delivers "Personal"'s identifier for that field

#### Scenario: A text area keeps line breaks

- **WHEN** the user types two lines into a text area and submits
- **THEN** the delivered value holds both lines with the line break

### Requirement: An action can report success with a toast

An action that keeps the launcher open SHALL be able to report success with a
one-line message. The launcher SHALL show it as a success toast.

#### Scenario: Pinning an entry

- **WHEN** the user runs an action that pins a clipboard entry and the action
  reports "Entry pinned"
- **THEN** a success toast reads "Entry pinned" and the view stays

### Requirement: The launcher filters lists and grids

The launcher SHALL narrow a list's or grid's items against the query as the
user types. A command SHALL NOT receive keystrokes as query changes and SHALL
NOT be invoked again because the query changed. A view SHALL NOT be able to
hand filtering to its command.

#### Scenario: Typing narrows a list

- **WHEN** a list is showing and the user types
- **THEN** the visible items are narrowed without the command being invoked
  again

### Requirement: A Shift or Alt shortcut names a letter or a named key

An action shortcut that includes Shift or Alt SHALL name a letter key, A to Z,
or a named key: Backspace, Tab, Enter, Escape, Delete, ArrowUp, ArrowDown,
ArrowLeft, ArrowRight, Home, End, PageUp, or PageDown. Dango SHALL refuse a
view tree in which any action declares Shift or Alt with any other key, such as
a digit, symbol, or punctuation key, and SHALL show an error in place of the
view, as for any other invalid tree. A shortcut without Shift or Alt MAY name
any key.

On macOS, Option with a letter and without Cmd or Ctrl types a composed
character, so a shortcut of Alt and a letter alone SHALL NOT be expected to
run there.

#### Scenario: A Shift shortcut on a letter runs

- **WHEN** a view declares an action with the shortcut Ctrl+Shift+P and the
  user presses Ctrl+Shift+P
- **THEN** that action runs

#### Scenario: A Shift shortcut on a named key runs

- **WHEN** a view declares an action with the shortcut Shift+Enter and the
  user presses Shift+Enter
- **THEN** that action runs

#### Scenario: An Alt shortcut on a named key runs

- **WHEN** a view declares an action with the shortcut Alt+ArrowUp and the
  user presses Alt+ArrowUp
- **THEN** that action runs

#### Scenario: A Shift shortcut on a digit is refused

- **WHEN** a command emits a view with an action whose shortcut is Shift+1
- **THEN** the view is rejected and an error is shown in its place

#### Scenario: An Alt shortcut on a symbol is refused

- **WHEN** a command emits a view with an action whose shortcut is Alt+/
- **THEN** the view is rejected and an error is shown in its place

#### Scenario: An Alt shortcut on a letter runs on Windows

- **WHEN** a view on Windows declares an action with the shortcut Alt+P and
  the user presses Alt+P
- **THEN** that action runs

#### Scenario: An Alt shortcut on a letter alone does not run on macOS

- **WHEN** a view on macOS declares an action with the shortcut Alt+P and the
  user presses Option+P
- **THEN** the action does not run

### Requirement: A list or grid shows at most 50 items

A list or grid view SHALL show at most 50 items. The limit SHALL apply after
the launcher has filtered the items against the query, and SHALL count items in
the order they are drawn, across all sections; a section left with no shown item
SHALL NOT be drawn. A command MAY send more items than the limit; the view SHALL
NOT be rejected for it. Items past the limit SHALL be reachable by typing a
query that narrows the list. Selection, the action panel, and the selected
item's detail SHALL only consider the items that are shown.

#### Scenario: A long list shows the first 50 items

- **WHEN** a command emits a list of 500 items and the query is empty
- **THEN** the first 50 items, in their declared order, are shown
- **AND** the rest are not shown

#### Scenario: Filtering reaches an item past the limit

- **WHEN** a list of 500 items is shown and the user types a query that only
  the 300th item matches
- **THEN** the 300th item is shown and can be selected

#### Scenario: The limit counts across sections

- **WHEN** a list has two sections of 40 items each and the query is empty
- **THEN** all 40 items of the first section and the first 10 items of the
  second section are shown

#### Scenario: A grid is bounded too

- **WHEN** a command emits a grid of 80 tiles and the query is empty
- **THEN** the first 50 tiles are shown

#### Scenario: Moving past the last shown item

- **WHEN** a list of 500 items is shown, the 50th item is selected, and the
  user moves the selection down
- **THEN** the 50th item stays selected
