## MODIFIED Requirements

### Requirement: Versioned protocol

The view protocol SHALL carry a `protocolVersion`. Dango SHALL reject a view
tree whose version it does not support and SHALL show an error rather than
rendering partially.

#### Scenario: Unsupported protocol version

- **WHEN** a command emits a view tree with an unsupported `protocolVersion`
- **THEN** an error is shown in place of the view
- **AND** the user can dismiss it and return to root

### Requirement: Full-tree replace

A command SHALL communicate a change by emitting a complete new view tree. The
protocol SHALL NOT define partial updates, patches, or diffs. Dango SHALL
identify list items by their identifier so that replacing a tree does not lose
selection or cause visible flicker.

#### Scenario: Selection survives a tree replacement

- **WHEN** the user has selected the third item and the command emits a new tree containing that same item identifier
- **THEN** the item remains selected

#### Scenario: Streaming content replaces the tree repeatedly

- **WHEN** a command replaces its view tree many times a second
- **THEN** the content updates without flicker or visible lag
- **AND** the selection and focus stay where they were

### Requirement: View stack

Views SHALL form a stack. Invoking an action that opens a view SHALL push onto
the stack, Escape or the SearchBar's back control SHALL pop one level, and
Escape at the root SHALL hide the launcher. When the launcher resets its state,
as launcher-shell describes, the stack SHALL be cleared back to root; an
interrupting hide within the grace period SHALL keep it. Each level SHALL keep its query, filter, and selection, so
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

- **WHEN** the user is three views deep, the launcher session ends with a
  deliberate hide or when the grace period runs out, and the user reopens the
  launcher
- **THEN** the root view is shown with an empty query

### Requirement: A detail can show an image

A detail, standalone or in a list's pane, SHALL be able to declare an image,
given as a named icon or a path to an image file. The image SHALL be shown
above the body, scaled down to fit the detail's width and a fixed maximum
height with its aspect ratio kept, and never scaled up. A detail without an image SHALL show only its body.

#### Scenario: Image above the body

- **WHEN** a detail declares an image and a body
- **THEN** the image is shown above the body

#### Scenario: A large image fits

- **WHEN** a detail declares an image wider than the detail
- **THEN** the image is scaled down to the detail's width, keeping its aspect ratio

#### Scenario: A tall image fits

- **WHEN** a detail declares an image taller than the maximum height
- **THEN** the image is scaled down to the maximum height, keeping its aspect
  ratio

#### Scenario: A small image is not enlarged

- **WHEN** a detail declares an image narrower than the detail
- **THEN** the image is shown at its own size

#### Scenario: An image that cannot be loaded

- **WHEN** a detail declares an image path that does not exist
- **THEN** the body and metadata are still shown, without an image


## REMOVED Requirements

### Requirement: Loading and empty states

**Reason**: Its "Reduced motion preserves loading feedback" scenario is from
the WebView era: nothing animates, so reduced motion changes nothing. The status
announcement rule is trimmed to what the launcher does: expose its busy state
to assistive technology.
A MODIFIED block cannot drop a scenario, so the rule moves to a new header.

**Migration**: See "A view shows loading and empty states".

## ADDED Requirements

### Requirement: A view shows loading and empty states

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

Busy state SHALL be exposed to assistive technology.

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

#### Scenario: Loading finishes

- **WHEN** the command emits a replacement with loading set to false
- **THEN** its content remains visible and the loading marks are removed
- **AND** assistive technology is no longer told that the view is busy

#### Scenario: Loading feedback is abandoned

- **WHEN** the user leaves a loading view, or the launcher session ends with a
  deliberate hide or when the grace period runs out (see launcher-shell)
- **THEN** its loading marks are removed
- **AND** a late update from an abandoned invocation does not restore them

### Requirement: A form with a multi-line field submits with a modifier

A text area and a template are multi-line fields. In a form with a multi-line
field, Enter in that field SHALL insert a line break and SHALL NOT submit the
form, and the ActionBar SHALL show the submit chord with the modifier: Cmd and
Enter on macOS, Ctrl and Enter on Windows. That chord SHALL submit any form,
from any field. In a form without a multi-line field, the ActionBar SHALL show
Enter alone.

#### Scenario: Enter in a text area on macOS

- **WHEN** a form on macOS has a text area with the cursor in it and the user
  presses Enter
- **THEN** a line break is inserted and the form is not submitted
- **AND** the ActionBar shows Cmd and Enter next to the submit action

#### Scenario: Enter in a text area on Windows

- **WHEN** a form on Windows has a text area with the cursor in it and the
  user presses Enter
- **THEN** a line break is inserted and the form is not submitted
- **AND** the ActionBar shows Ctrl and Enter next to the submit action

#### Scenario: Submitting a form with a text area on macOS

- **WHEN** a form on macOS has a text area and the user presses Cmd+Enter
- **THEN** the form is submitted once, with the text area's line breaks kept

#### Scenario: Submitting a form with a text area on Windows

- **WHEN** a form on Windows has a text area and the user presses Ctrl+Enter
- **THEN** the form is submitted once, with the text area's line breaks kept

#### Scenario: A form without a multi-line field

- **WHEN** a form has only single-line fields
- **THEN** the ActionBar shows Enter alone and Enter submits the form
