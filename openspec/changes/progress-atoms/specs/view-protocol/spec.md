# Spec Delta

## MODIFIED Requirements

### Requirement: Items can carry accessories and keywords

A list item SHALL be able to declare up to three accessories, each one of text,
an icon, a tag with a role of accent, muted, or danger, a key chord, a progress
ring with an optional value from 0 to 1, or a status of working, needs input,
done, failed, or queued. A progress ring without a value SHALL be shown as
indeterminate. Dango SHALL refuse a view tree in which a progress value lies
outside 0 to 1, and SHALL show an error in place of the view. An item SHALL be
able to declare keywords. When the launcher filters, a query SHALL match an
item by its keywords as well as its title, and the keywords SHALL NOT be shown.

#### Scenario: Accessories are shown

- **WHEN** an item declares the text "2m" and a tag "Link" in the accent role
- **THEN** the row shows "2m" and the "Link" tag on its right

#### Scenario: Progress and status accessories

- **WHEN** an item declares a progress ring at 0.42 and the status working
- **THEN** the row shows the ring filled to 42 percent and a working dot on its
  right

#### Scenario: A progress value out of range

- **WHEN** an item declares a progress ring at 1.5
- **THEN** Dango shows an error in place of the view

#### Scenario: A keyword matches

- **WHEN** an item titled "Terminal" declares the keyword "shell" and the user
  types "shell"
- **THEN** the item is shown

### Requirement: A view shows loading and empty states

A view SHALL be able to signal that it is loading and to declare what is shown
when it has no content.

Every list, grid, or detail view marked loading SHALL show the SearchBar's
loading bar and a busy toast reading "Loading…", whether or not it already
contains content. These loading marks SHALL appear only once the view has been
loading for about 150 milliseconds, and once shown SHALL stay for at least
about 400 milliseconds, even if loading ends sooner. Entering the loading state
SHALL NOT itself clear, replace, dim into unreadability, or block interaction
with content supplied by the command. The loading marks SHALL reflect the
declared loading state, not infer completion from incoming text, and SHALL NOT
claim a numerical progress value.

A list marked loading with no items SHALL show skeleton rows (see
design-system) from the moment it is shown, without waiting for the loading
marks.

An empty state SHALL have a title and MAY have an icon, a one-line description,
and actions. When an empty view is shown, Enter SHALL run its first action.

Busy state SHALL be exposed to assistive technology while the loading marks are
shown.

#### Scenario: Loading is visible

- **WHEN** a command emits a view marked as loading and keeps it loading
- **THEN** after a short moment the loading bar and the busy toast are shown
  without clearing any content already displayed

#### Scenario: A fast load shows no loading marks

- **WHEN** a command emits a view marked loading and replaces it with one not
  marked loading within about 150 milliseconds
- **THEN** neither the loading bar nor the busy toast is shown

#### Scenario: Loading marks do not blink

- **WHEN** the loading marks have just appeared and the command then emits a
  view not marked loading
- **THEN** the marks stay until they have been shown for about 400
  milliseconds, and are then removed

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
- **THEN** the list shows skeleton rows at once, the busy toast reads
  "Loading…" once the loading marks appear, and the completed empty state is
  not shown

#### Scenario: Populated list is refreshing

- **WHEN** a command emits a loading list with existing items
- **THEN** those items remain visible and selectable, and the loading marks are
  shown
- **AND** the existing item-identity rules continue to preserve selection

#### Scenario: Detail content is streaming

- **WHEN** a command repeatedly replaces a detail view marked loading
- **THEN** the supplied content remains visible alongside the loading marks

#### Scenario: Loading finishes

- **WHEN** the command emits a replacement with loading set to false and the
  loading marks have been shown for at least about 400 milliseconds
- **THEN** its content remains visible and the loading marks are removed
- **AND** assistive technology is no longer told that the view is busy

#### Scenario: Loading feedback is abandoned

- **WHEN** the user leaves a loading view, or the launcher resets through a
  deliberate hide or the grace period running out (see launcher-shell)
- **THEN** its loading marks are removed at once, without waiting for the
  minimum time
- **AND** a late update from an abandoned invocation does not restore them
