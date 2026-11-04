## MODIFIED Requirements

### Requirement: Every screen is one shell around one view

The launcher SHALL show, from top to bottom, the SearchBar, exactly one view,
and the ActionBar. The view is a list, a grid, a detail, a form, a progress
view, or an empty view. In root search, an answer card MAY sit between the
SearchBar and the results; when no result matches, the card alone stands in
for the empty view. Overlays SHALL float above these regions and SHALL NOT
move them.

The SearchBar SHALL be a pill on its own. The view and the ActionBar SHALL sit
together in one card below it, separated from the pill by a gap through which
what is behind the launcher shows. At root with an empty query the card MAY be
hidden, as root-search describes; the pill alone is then the launcher.

The pill and the card SHALL each be 720 logical pixels wide. The pill SHALL be
52 logical pixels high and fully rounded at its ends, the gap 8, the ActionBar
44, and a list row 44.

#### Scenario: Root search

- **WHEN** the user types a query at root
- **THEN** the launcher shows the SearchBar pill and, below the gap, one card
  holding a list of results and the ActionBar

#### Scenario: Sizes at default text scale

- **WHEN** root search shows results at the default text scale
- **THEN** the pill and the card are 720 logical pixels wide, the pill is 52
  high, the gap 8, each result row 44, and the ActionBar 44

#### Scenario: The gap shows what is behind

- **WHEN** the launcher shows the pill and the card over another application
- **THEN** that application is visible through the gap between them

#### Scenario: Root search with an answer

- **WHEN** the user types "2+2" at root
- **THEN** the card shows the answer card, the results, and the ActionBar, from
  top to bottom

#### Scenario: A pushed view

- **WHEN** a command pushes a list, grid, detail, or form
- **THEN** the pill and the card are both shown

### Requirement: The SearchBar holds the query and shows where the user is

The SearchBar SHALL hold the query for a list or grid, with a placeholder the
view sets. When a view has been pushed, it SHALL show a back control before
the query, drawn as a chevron in a small circle; activating it SHALL go back
one view, as Escape does. While the busy marks are shown, it SHALL show an
indeterminate progress bar along its bottom edge: a thin bar in which a short
segment in the accent role sweeps from one end to the other, again and again,
at a steady pace. When the view declares a filter dropdown, the SearchBar
SHALL show it at its right end as a pill. For a detail or form view, the
SearchBar SHALL show the command's title in place of the query.

#### Scenario: Root shows no back control

- **WHEN** the launcher is at root
- **THEN** the SearchBar shows no back control

#### Scenario: A pushed view shows the back control

- **WHEN** a command has pushed a list
- **THEN** the SearchBar shows a back control, a chevron in a small circle,
  before the query

#### Scenario: Clicking back

- **WHEN** the user clicks the back control
- **THEN** the previous view is shown with its query and selection intact

#### Scenario: A filter dropdown

- **WHEN** a pushed list declares a filter dropdown
- **THEN** the SearchBar shows the selected option as a pill at its right end

#### Scenario: A loading view

- **WHEN** the busy marks are shown for a view marked loading
- **THEN** the loading bar is shown along the bottom of the SearchBar pill
  with its segment sweeping along it, inside the pill's rounded outline

#### Scenario: Loading ends

- **WHEN** the busy marks are removed, or the user leaves the view
- **THEN** the loading bar and its motion are gone in the same frame

#### Scenario: A form shows its title

- **WHEN** a command pushes a form
- **THEN** the SearchBar shows that command's title and accepts no typing

### Requirement: The ActionBar answers what Enter does

The ActionBar SHALL sit at the bottom of the card. It SHALL show, on its left,
the context: in root search the name and icon of the extension that
contributed the selected result, and in a pushed view the command's title and
icon. On its right it SHALL show the label of the action Enter runs with the
Enter key, together as one chip on a raised fill, and, when the selection has
more than one action, the word "Actions" with the action panel's chord. When
nothing can be confirmed, the primary action and its key SHALL be hidden.
Clicking "Actions" SHALL open the action panel.

#### Scenario: An application is selected in root search

- **WHEN** an application result is selected in root search
- **THEN** the ActionBar shows "Applications" with its icon on the left and a
  chip reading "Open" with the Enter key on the right

#### Scenario: Inside a pushed view

- **WHEN** clipboard history is open
- **THEN** the ActionBar's context is "Clipboard History" with its icon

#### Scenario: A single action

- **WHEN** the selected item has exactly one action
- **THEN** the ActionBar shows the primary action chip and no "Actions" hint

#### Scenario: Nothing to confirm

- **WHEN** a list is empty and its empty view declares no action
- **THEN** the ActionBar shows only its context

### Requirement: A toast reports status in the ActionBar

A toast SHALL replace the ActionBar's context while it shows. When the card is
hidden and only the pill shows, the toast SHALL instead show at the pill's
right end, with the same style, text, and action, cut off with an ellipsis
before it reaches the query. A toast
has a style and one line of text, and MAY carry one action, shown after the
text as its label and chord. Pressing that chord while the toast shows SHALL
run the action and SHALL NOT also act on the view. A busy toast SHALL show a
spinner in the working role and stay while its work runs, following the
busy-mark timing. A success toast SHALL show a dot in the done role and
disappear after about two seconds, unless it carries an action: then it SHALL
stay until the next key press or click, so the action is still there when the
user looks. A failure toast SHALL show a dot in the danger role and stay until
the next key press or click. The success and failure dots SHALL NOT move.
Assistive technology SHALL be told the toast's text, its action, and whether
it reports a failure. Showing a toast SHALL NOT take focus.

#### Scenario: A success toast

- **WHEN** an action that keeps the launcher open reports success with a toast
- **THEN** the toast shows with a dot in the done role and is gone after about
  two seconds, and the context returns

#### Scenario: A toast on the bare pill

- **WHEN** a toast shows while the launcher is the pill alone
- **THEN** it shows at the pill's right end, the card stays hidden, and the
  launcher does not change size

#### Scenario: The card opens while a toast shows on the pill

- **WHEN** a toast shows at the pill's end and the user types a character
- **THEN** the card opens and the toast, if it is still showing, moves into
  the ActionBar's context

#### Scenario: A success toast with an action stays

- **WHEN** a success toast reads "Answer ready: Summarize" with "Show answer"
- **THEN** it stays until the user presses a key or clicks

#### Scenario: A failure toast stays

- **WHEN** an action fails
- **THEN** a failure toast shows its sentence with a dot in the danger role
- **AND** it stays until the user presses a key or clicks

#### Scenario: A failure toast with an action on Windows

- **WHEN** a failure toast carries "Retry" and the user presses Ctrl+R
- **THEN** the retry runs once and the toast is gone

#### Scenario: A failure toast with an action on macOS

- **WHEN** a failure toast carries "Retry" and the user presses Cmd+R
- **THEN** the retry runs once and the toast is gone

#### Scenario: A busy toast

- **WHEN** the busy marks are shown
- **THEN** the busy toast shows a spinner in the working role, turning while
  the work runs

#### Scenario: Typing is not interrupted

- **WHEN** a toast appears while the user types in the SearchBar
- **THEN** every character still reaches the SearchBar

