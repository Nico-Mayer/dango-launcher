## Purpose

Define how the launcher looks and how its parts are composed: the theme roles,
the light and dark themes, the type faces, the shell regions, the views, the
overlays, and the small parts every surface is built from, so that every screen
is one shell around one view and reads like every other.

## Requirements

### Requirement: The theme follows the operating system

Dango SHALL have two themes, Graphite, which is dark, and Paper, which is light,
and SHALL show the one that matches the operating system's appearance setting.
When the setting changes, the launcher SHALL use the new theme the next time it
is painted, without a restart. Dango SHALL NOT read or create a theme file, and
SHALL NOT offer a theme picker.

#### Scenario: Dark appearance on Windows

- **WHEN** Windows is set to the dark app mode and the launcher is shown
- **THEN** the launcher uses the Graphite theme

#### Scenario: Light appearance on Windows

- **WHEN** Windows is set to the light app mode and the launcher is shown
- **THEN** the launcher uses the Paper theme

#### Scenario: Dark appearance on macOS

- **WHEN** macOS is set to the dark appearance and the launcher is shown
- **THEN** the launcher uses the Graphite theme

#### Scenario: Light appearance on macOS

- **WHEN** macOS is set to the light appearance and the launcher is shown
- **THEN** the launcher uses the Paper theme

#### Scenario: The setting changes while Dango runs

- **WHEN** the operating system's appearance changes while Dango is running and
  the user then shows the launcher
- **THEN** the launcher uses the matching theme without Dango being restarted

### Requirement: Dango's own colours come from the active theme

Every colour Dango draws itself, its surfaces, text, lines, and marks, SHALL
come from the active theme. Content brings its own colours and keeps them:
application icons, favicons, image previews, and colour tiles a command
declares. The two themes SHALL differ only in colour, never in the size or
position of any part.

#### Scenario: Switching theme changes Dango's colours

- **WHEN** the operating system's appearance changes from dark to light and the
  launcher is shown
- **THEN** the launcher's surfaces, text, and selection use the light theme

#### Scenario: Content keeps its colours

- **WHEN** a grid shows a colour tile declared as `#ff0000` in either theme
- **THEN** the tile is drawn in `#ff0000`

#### Scenario: Switching theme keeps the layout

- **WHEN** the same results are shown once in the dark theme and once in the
  light theme
- **THEN** every part has the same size and position in both

### Requirement: Text uses the bundled type faces

The launcher SHALL set its text in Geist, and key chords and code in Geist Mono.
Both SHALL ship with Dango, so the launcher looks the same whether or not they
are installed on the system. A character neither face holds SHALL fall back to
a system face rather than show as a missing glyph.

#### Scenario: Fonts are not installed

- **WHEN** Dango runs on a system without Geist or Geist Mono installed
- **THEN** result titles are set in Geist and key chords in Geist Mono

#### Scenario: A glyph the face lacks

- **WHEN** a result title contains a character Geist does not hold
- **THEN** that character is drawn from a system face, not as a box

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
the query, drawn as a back arrow in a small circle; activating it SHALL go back
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
- **THEN** the SearchBar shows a back control, a back arrow in a small circle,
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

### Requirement: A ticker reports status in the ActionBar

A ticker SHALL replace the ActionBar's context while it shows. When the card is
hidden and only the pill shows, the ticker SHALL instead show at the pill's
right end, with the same style, text, and action, cut off with an ellipsis
before it reaches the query. A ticker
has a style and one line of text, and MAY carry one action, shown after the
text as its label and chord. Pressing that chord while the ticker shows SHALL
run the action and SHALL NOT also act on the view. A busy ticker SHALL show a
spinner in the working role and stay while its work runs, following the
busy-mark timing. A success ticker SHALL show a dot in the done role and
disappear after about two seconds, unless it carries an action: then it SHALL
stay until the next key press or click, so the action is still there when the
user looks. A failure ticker SHALL show a dot in the danger role and stay until
the next key press or click. The success and failure dots SHALL NOT move.
Assistive technology SHALL be told the ticker's text, its action, and whether
it reports a failure. Showing a ticker SHALL NOT take focus.

#### Scenario: A success ticker

- **WHEN** an action that keeps the launcher open reports success with a ticker
- **THEN** the ticker shows with a dot in the done role and is gone after about
  two seconds, and the context returns

#### Scenario: A ticker on the bare pill

- **WHEN** a ticker shows while the launcher is the pill alone
- **THEN** it shows at the pill's right end, the card stays hidden, and the
  launcher does not change size

#### Scenario: The card opens while a ticker shows on the pill

- **WHEN** a ticker shows at the pill's end and the user types a character
- **THEN** the card opens and the ticker, if it is still showing, moves into
  the ActionBar's context

#### Scenario: A success ticker with an action stays

- **WHEN** a success ticker reads "Answer ready: Summarize" with "Show answer"
- **THEN** it stays until the user presses a key or clicks

#### Scenario: A failure ticker stays

- **WHEN** an action fails
- **THEN** a failure ticker shows its sentence with a dot in the danger role
- **AND** it stays until the user presses a key or clicks

#### Scenario: A failure ticker with an action on Windows

- **WHEN** a failure ticker carries "Retry" and the user presses Ctrl+R
- **THEN** the retry runs once and the ticker is gone

#### Scenario: A failure ticker with an action on macOS

- **WHEN** a failure ticker carries "Retry" and the user presses Cmd+R
- **THEN** the retry runs once and the ticker is gone

#### Scenario: A busy ticker

- **WHEN** the busy marks are shown
- **THEN** the busy ticker shows a spinner in the working role, turning while
  the work runs

#### Scenario: Typing is not interrupted

- **WHEN** a ticker appears while the user types in the SearchBar
- **THEN** every character still reaches the SearchBar

### Requirement: Small parts look the same everywhere

The launcher SHALL build every surface from the same small parts:

- An icon is 20 logical pixels, square or round. A named icon is drawn in a
  theme role. An application icon or favicon keeps its own colours and aspect
  ratio. A content preview, such as a copied image, fills its frame and has a
  border.
- A key chord is drawn as keys in Geist Mono, using the platform's names as the
  view protocol requires, in a chip that fits its text. On Windows the keys are
  separated by a space; on macOS they are joined with none. Every key chord
  uses this one format: the footer, the action panel, tickers, and result rows.
- A tag is short text in its role's colour on a rounded, faint fill of the same
  role. Its role is accent, muted, or danger.
- A progress ring as an accessory is 16 logical pixels.
- An accessory is text, an icon, a tag, a key chord, a progress ring, or a
  status dot. A row shows at most three, right-aligned.

#### Scenario: Icons keep their meaning

- **WHEN** results include named icons, application icons, favicons, and image
  previews
- **THEN** named icons follow the theme, application icons and favicons keep
  their colours and are not stretched, and image previews are framed

#### Scenario: Key chords on Windows

- **WHEN** the launcher shows the action panel's chord, a ticker's "Show answer"
  chord, and a result's hotkey on Windows
- **THEN** they read "Ctrl K", "Ctrl ↵", and "Ctrl Alt V"

#### Scenario: Key chords on macOS

- **WHEN** the launcher shows the action panel's chord, a ticker's "Show answer"
  chord, and a result's hotkey on macOS
- **THEN** they read "⌘K", "⌘↵", and "⌃⌥V"

#### Scenario: A fourth accessory

- **WHEN** an item declares four accessories
- **THEN** the row shows the first three

#### Scenario: A tag in the accent role

- **WHEN** an item declares the tag "Link" in the accent role
- **THEN** "Link" is drawn in the accent colour on a faint accent fill, with no
  outline

### Requirement: Each view kind has one layout

- A list SHALL show its rows under optional section headings. A heading shows
  its title and, when given, its subtitle such as a count. With a detail pane,
  the list SHALL take two fifths of the width and the detail of the selected
  item the rest.
- A grid SHALL show tiles in a fixed number of columns at the declared aspect
  ratio, each with its title below, under optional section headings. The
  selected tile SHALL be outlined in the accent role.
- A detail SHALL show its image, when declared, framed above its body, then
  its body, and, when declared, its metadata as label and value pairs at the
  bottom of the detail, labels in the muted role. The metadata SHALL stay in
  view while a long body scrolls above it. A plain text body SHALL use the
  monospace face; the metadata SHALL keep the interface face, so it does not
  read as part of the body.
- A form SHALL show a label column and a control column. A field's error SHALL
  be shown under the field in the danger role and be its accessible
  description.
- A progress view SHALL show, from top to bottom, its title with its subtitle
  in the muted role, a 4 logical pixel progress bar, its detail line in the
  muted role, and its steps, one per row, each with a status dot before its
  title. Queued steps' titles SHALL be in the muted role.
- An empty view SHALL show one icon, one title, and at most one line of hint,
  centred in the view.

#### Scenario: A list with a detail pane

- **WHEN** a list declares a detail pane and a row is selected
- **THEN** the list fills the left two fifths and the selected item's detail the
  right three fifths

#### Scenario: A detail with an image

- **WHEN** a detail declares an image, a body, and metadata
- **THEN** the image is framed at the top, the body follows, and the metadata
  is at the bottom of the detail

#### Scenario: A short body leaves the metadata at the bottom

- **WHEN** a detail declares a one-line body and metadata
- **THEN** the metadata sits at the bottom of the detail, not directly under
  the body

#### Scenario: A long body scrolls above the metadata

- **WHEN** a detail declares a body taller than the detail, and metadata
- **THEN** the body scrolls and the metadata stays in view at the bottom

#### Scenario: A plain text body

- **WHEN** a detail declares plain text
- **THEN** the text is shown in the monospace face

#### Scenario: A form error

- **WHEN** a form field reports an error
- **THEN** its message shows under that field in the danger role and is exposed
  as the field's description, and the field can still be edited

#### Scenario: A progress view

- **WHEN** a progress view declares a title, a subtitle, a value, a detail, and
  four steps
- **THEN** the title and subtitle are at the top, the bar below them, then the
  detail line, then the four steps with their status dots

#### Scenario: No results

- **WHEN** a query matches nothing
- **THEN** the empty view shows its icon, "No results for" followed by the
  query, and its hint when one is declared

### Requirement: Selection, hover, and focus are told apart

The selected row, tile, or action SHALL be filled with the selected role. A
hovered one SHALL be filled with the hover role, which SHALL be clearly weaker
than selected, following the list-navigation requirements. Selected text in an
input SHALL be filled with the selection role. A focused form control SHALL
show a border in the accent role. A surface that keeps focus in its input and
moves a selection SHALL show that selection, not a focus ring around the
surface.

#### Scenario: Hovering another row

- **WHEN** a row is selected and the pointer rests on another
- **THEN** the hovered row is clearly weaker than the selected row

#### Scenario: Tabbing through a form

- **WHEN** the user presses Tab in a form
- **THEN** the next control shows the accent border and the previous one loses
  it

#### Scenario: Selecting text in the SearchBar

- **WHEN** the user selects the query text in the SearchBar
- **THEN** the selected text is filled with the selection role, not the
  selected role

### Requirement: Overlays own the keyboard until they close

The action panel SHALL float above the ActionBar at the launcher's right edge,
with a shadow. While it is open it SHALL own navigation, typing, and
confirmation, and the selection under it SHALL stay unchanged. Closing it SHALL
return focus to where it was. The key or click that closes it SHALL NOT also
act on the view below or dismiss the launcher.

#### Scenario: Open and close the action panel on Windows

- **WHEN** the user opens the panel with Ctrl+K, moves through it with the
  arrow keys, and presses Escape
- **THEN** only the panel closes, the selection below is unchanged, and typing
  reaches the SearchBar or form control again

#### Scenario: Open and close the action panel on macOS

- **WHEN** the user opens the panel with Cmd+K or Ctrl+K, moves through it with
  the arrow keys, and presses Escape
- **THEN** only the panel closes, the selection below is unchanged, and typing
  reaches the SearchBar or form control again

#### Scenario: Confirming in the panel runs one action

- **WHEN** the user confirms an action in the panel
- **THEN** that action runs once and the item below is not also confirmed

#### Scenario: A detail view gets the keyboard back

- **WHEN** the panel closes over a detail view
- **THEN** the detail view's shortcuts work again

### Requirement: Content fits the launcher

Content and overlays SHALL fit inside the launcher at every supported display
scaling. Long text SHALL be cut off with an ellipsis or wrapped, and SHALL NOT
push the ActionBar or controls out of view or scroll sideways. A long action
list SHALL scroll and keep the selected action in view.

#### Scenario: Long titles and labels

- **WHEN** results, actions, tickers, and form help are longer than their space
- **THEN** they are cut off or wrapped, the ActionBar stays in view, and nothing
  scrolls sideways

#### Scenario: Long action list

- **WHEN** the panel has more actions than fit and the user moves to the last
- **THEN** the panel scrolls it into view and the selection below does not move

#### Scenario: High display scaling

- **WHEN** the launcher is shown at 200 percent scaling on either platform
- **THEN** corners, icons, text, and focus marks are sharp and not clipped by
  the window edge

### Requirement: Progress marks are honest

A progress mark SHALL be a bar or a ring, filled in the working role on a track
in the raised role. It SHALL be determinate, filled to its value from 0 to 1,
only when the total is known. Otherwise it SHALL be indeterminate and move
without claiming an amount. A spinner is a small indeterminate ring. The
SearchBar's loading bar is the one exception to these colours (see its
requirement). A mark SHALL tell assistive technology that it is busy and, when
determinate, its value in percent.

#### Scenario: A determinate ring

- **WHEN** an item declares a progress ring at 0.42
- **THEN** the ring is filled to 42 percent and does not move
- **AND** assistive technology is told the value 42 percent

#### Scenario: A full ring

- **WHEN** an item declares a progress ring at 1
- **THEN** the ring is drawn as a full circle

#### Scenario: An indeterminate ring

- **WHEN** an item declares a progress ring with no value
- **THEN** the ring turns without showing an amount

### Requirement: Status dots name a state

A status dot SHALL show one of five states, each in its own role: working in
the working role, needs input in the blocked role, done in the done role,
failed in the danger role, and queued in the idle role. A status dot SHALL NOT
move. A status dot shown as an item's accessory SHALL tell assistive
technology its state as "Working", "Needs input", "Done", "Failed", or
"Queued".

#### Scenario: A failed item

- **WHEN** an item declares the status failed
- **THEN** its row shows a dot in the danger role
- **AND** assistive technology is told "Failed"

### Requirement: A cold list shows skeleton rows

A list whose view is loading and declares no items SHALL show skeleton rows in
its body: rows of the normal row height, with a shape in the raised role where
the icon and title would be, as many as fit the launcher at its maximum height.
Skeleton rows SHALL NOT be selectable, SHALL NOT be read out one by one to
assistive technology, and SHALL be replaced by the items in the same frame they
arrive. A list whose items are all filtered out by the query SHALL NOT show
skeleton rows.

#### Scenario: Opening a slow list

- **WHEN** a command shows a list marked loading with no items
- **THEN** the launcher takes its maximum height and the body shows skeleton
  rows and no empty state

#### Scenario: Items arrive

- **WHEN** the first items of that list arrive
- **THEN** the skeleton rows are gone and the items are shown in the same frame

#### Scenario: A refreshing list filtered to nothing

- **WHEN** a list marked loading has items and the query matches none of them
- **THEN** no skeleton rows are shown

### Requirement: Busy marks do not blink

The busy marks, the SearchBar's loading bar and the busy ticker, SHALL appear
only once Dango has been busy for about 150 milliseconds. Dango is busy while a
command invoked from root search has not finished or shown a view, while the
top view is marked loading, or while a background task runs and its progress
view is not shown. A running background task SHALL show only the busy ticker,
never the loading bar. Once shown, the marks SHALL stay for at least
about 400 milliseconds, even if the work ends sooner. While Dango stays busy,
replacing the view SHALL NOT restart the marks. Leaving the view or a reset
SHALL remove them at once.

#### Scenario: A fast command

- **WHEN** a command invoked from root search finishes within about 150
  milliseconds
- **THEN** no busy ticker is shown

#### Scenario: A slow command shows a view that loads

- **WHEN** a command takes a second to show a view marked loading, and the
  view loads for another second
- **THEN** the busy ticker appears after a short moment reading "Running"
  followed by the command's title, then reads "Loading…" without disappearing
  in between

#### Scenario: Busy marks stay a moment

- **WHEN** the busy marks have just appeared and the work then ends
- **THEN** the marks stay until they have been shown for about 400
  milliseconds, and are then removed

#### Scenario: A streamed view keeps its marks steady

- **WHEN** a command replaces a view marked loading many times a second
- **THEN** the busy marks stay shown and the loading bar's sweep continues
  without restarting

#### Scenario: Leaving clears at once

- **WHEN** the user leaves a view while its busy marks are shown
- **THEN** the marks are gone in the same frame

#### Scenario: A background task shows no loading bar

- **WHEN** an update downloads and the user opens clipboard history
- **THEN** the busy ticker shows the download and the SearchBar shows no
  loading bar
