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
and the ActionBar. The view is a list, a grid, a detail, a form, or an empty
view. Overlays SHALL float above these regions and SHALL NOT move them.

The launcher SHALL be 720 logical pixels wide. The SearchBar SHALL be 64
logical pixels high, the ActionBar 44, and a list row 44.

#### Scenario: Root search

- **WHEN** the launcher opens at root
- **THEN** it shows the SearchBar, a list of results, and the ActionBar

#### Scenario: Sizes at default text scale

- **WHEN** root search is shown at the default text scale
- **THEN** the launcher is 720 logical pixels wide, the SearchBar is 64 high,
  each result row is 44 high, and the ActionBar is 44 high

### Requirement: The SearchBar holds the query and shows where the user is

The SearchBar SHALL hold the query for a list or grid, with a placeholder the
view sets. When a view has been pushed, it SHALL show a back control before
the query; activating it SHALL go back one view, as Escape does. While the view
is loading, it SHALL show a thin bar along its bottom edge in which a short
segment in the accent role sweeps from one end to the other, again and again,
at a steady pace. When the view declares a filter dropdown, the SearchBar SHALL
show it at its right end. For a detail or form view, the SearchBar SHALL show
the command's title in place of the query.

#### Scenario: Root shows no back control

- **WHEN** the launcher is at root
- **THEN** the SearchBar shows no back control

#### Scenario: A pushed view shows the back control

- **WHEN** a command has pushed a list
- **THEN** the SearchBar shows a back control before the query

#### Scenario: Clicking back

- **WHEN** the user clicks the back control
- **THEN** the previous view is shown with its query and selection intact

#### Scenario: A loading view

- **WHEN** the view is marked loading
- **THEN** the loading bar is shown under the SearchBar with its segment
  sweeping along it

#### Scenario: Loading ends

- **WHEN** the view stops being marked loading, or the user leaves it
- **THEN** the loading bar and its motion are gone in the same frame

#### Scenario: A form shows its title

- **WHEN** a command pushes a form
- **THEN** the SearchBar shows that command's title and accepts no typing

### Requirement: The ActionBar answers what Enter does

The ActionBar SHALL show, on its left, the context: in root search the name and
icon of the extension that contributed the selected result, and in a pushed
view the command's title and icon. On its right it SHALL show the label of the
action Enter runs with the Enter key, and, when the selection has more than one
action, the word "Actions" with the action panel's chord. When nothing can be
confirmed, the primary action and its key SHALL be hidden. Clicking "Actions"
SHALL open the action panel.

#### Scenario: An application is selected in root search

- **WHEN** an application result is selected in root search
- **THEN** the ActionBar shows "Applications" with its icon on the left and
  "Open" with the Enter key on the right

#### Scenario: Inside a pushed view

- **WHEN** clipboard history is open
- **THEN** the ActionBar's context is "Clipboard History" with its icon

#### Scenario: A single action

- **WHEN** the selected item has exactly one action
- **THEN** the ActionBar shows the primary action and no "Actions" hint

#### Scenario: Nothing to confirm

- **WHEN** a list is empty and its empty view declares no action
- **THEN** the ActionBar shows only its context

### Requirement: A toast reports status in the ActionBar

A toast SHALL replace the ActionBar's context while it shows. A toast has a
style and one line of text. A busy toast SHALL show a dot in the working role
and stay while its work runs. A success toast SHALL show a dot in the done role
and disappear after about two seconds. A failure toast SHALL show a dot in the
danger role and stay until the next key press or click. The dot SHALL NOT move.
Assistive technology SHALL be told the toast's text and whether it reports a
failure. Showing a toast SHALL NOT take focus.

#### Scenario: A success toast

- **WHEN** an action that keeps the launcher open reports success with a toast
- **THEN** the toast shows with a dot in the done role and is gone after about
  two seconds, and the context returns

#### Scenario: A failure toast stays

- **WHEN** an action fails
- **THEN** a failure toast shows its sentence with a dot in the danger role
- **AND** it stays until the user presses a key or clicks

#### Scenario: A busy toast

- **WHEN** a view is marked loading
- **THEN** the busy toast shows a dot in the working role

#### Scenario: Typing is not interrupted

- **WHEN** a toast appears while the user types in the SearchBar
- **THEN** every character still reaches the SearchBar

### Requirement: Small parts look the same everywhere

The launcher SHALL build every surface from the same small parts:

- An icon is 20 logical pixels, square or round. A named icon is drawn in a
  theme role. An application icon or favicon keeps its own colours and aspect
  ratio. A content preview, such as a copied image, fills its frame and has a
  border.
- A key chord is drawn as keys in Geist Mono, using the platform's names as the
  view protocol requires.
- A tag is short text in its role's colour on a rounded, faint fill of the same
  role. Its role is accent, muted, or danger.
- An accessory is text, an icon, a tag, or a key chord. A row shows at most
  three, right-aligned.

#### Scenario: Icons keep their meaning

- **WHEN** results include named icons, application icons, favicons, and image
  previews
- **THEN** named icons follow the theme, application icons and favicons keep
  their colours and are not stretched, and image previews are framed

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

### Requirement: Nothing animates

Dango SHALL NOT animate its interface, except for the loading bar's sweep.
Showing the launcher, moving the selection, hovering, opening and closing the
action panel, showing and removing a toast, changing results, and replacing a
view SHALL each change in one frame. Busy states SHALL be shown by the loading
bar and by static marks next to text. How fast the launcher appears is in
launcher-shell, and how a streamed view keeps its selection is in
view-protocol.

#### Scenario: Activation on Windows

- **WHEN** the launcher is shown on Windows, first or again
- **THEN** it appears without an entrance effect

#### Scenario: Activation on macOS

- **WHEN** the launcher is shown on macOS, first or again
- **THEN** it appears without an entrance effect

#### Scenario: A streamed view stays stable

- **WHEN** a command replaces one view many times a second
- **THEN** each replacement shows in one frame, with no transition between the
  old and the new content
- **AND** the loading bar's sweep continues without restarting

#### Scenario: Busy without motion

- **WHEN** work is in progress
- **THEN** the loading bar's segment is the only thing that moves
- **AND** the busy toast's dot stays still

#### Scenario: Idle without motion

- **WHEN** no view is loading
- **THEN** no animation runs on any surface

### Requirement: Content fits the launcher

Content and overlays SHALL fit inside the launcher at every supported display
scaling. Long text SHALL be cut off with an ellipsis or wrapped, and SHALL NOT
push the ActionBar or controls out of view or scroll sideways. A long action
list SHALL scroll and keep the selected action in view.

#### Scenario: Long titles and labels

- **WHEN** results, actions, toasts, and form help are longer than their space
- **THEN** they are cut off or wrapped, the ActionBar stays in view, and nothing
  scrolls sideways

#### Scenario: Long action list

- **WHEN** the panel has more actions than fit and the user moves to the last
- **THEN** the panel scrolls it into view and the selection below does not move

#### Scenario: High display scaling

- **WHEN** the launcher is shown at 200 percent scaling on either platform
- **THEN** corners, icons, text, and focus marks are sharp and not clipped by
  the window edge
