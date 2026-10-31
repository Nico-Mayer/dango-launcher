## Purpose

Define how the launcher looks and how its parts are composed: the theme roles,
the light and dark themes, the type faces, the shell regions, the views, the
overlays, and the small parts every surface is built from, so that every screen
is one shell around one view and reads like every other.

## ADDED Requirements

### Requirement: Every surface takes its look from one theme

Every colour, corner radius, row height, gap, and type size on every launcher
surface SHALL come from one theme. The theme SHALL have these colour roles:
background, surface, raised, border, text, muted, accent, selection, hover, and
danger. A surface SHALL use a role, never a colour of its own, so changing a
role's value changes every surface that uses it, the action panel included.

#### Scenario: Changing the selection role

- **WHEN** the value of the selection role is changed
- **THEN** the selected row in root search, in a pushed list, in a grid, and in
  the action panel all use the new value
- **AND** the hover, accent, and danger roles keep their own values

#### Scenario: Floating content follows the theme

- **WHEN** the radius, the surface role, or a type size is changed and the
  action panel opens
- **THEN** the panel uses the changed values

### Requirement: The theme follows the operating system

Dango SHALL have a dark and a light theme and SHALL show the one that matches
the operating system's appearance setting. When the setting changes, the
launcher SHALL use the new theme the next time it is painted, without a
restart. Dango SHALL NOT read or create a theme file, and SHALL NOT offer a
theme picker.

#### Scenario: Dark appearance on Windows

- **WHEN** Windows is set to the dark app mode and the launcher is shown
- **THEN** the launcher uses the dark theme

#### Scenario: Light appearance on Windows

- **WHEN** Windows is set to the light app mode and the launcher is shown
- **THEN** the launcher uses the light theme

#### Scenario: Dark appearance on macOS

- **WHEN** macOS is set to the dark appearance and the launcher is shown
- **THEN** the launcher uses the dark theme

#### Scenario: Light appearance on macOS

- **WHEN** macOS is set to the light appearance and the launcher is shown
- **THEN** the launcher uses the light theme

#### Scenario: The setting changes while Dango runs

- **WHEN** the operating system's appearance changes while Dango is running and
  the user then shows the launcher
- **THEN** the launcher uses the matching theme without Dango being restarted

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

The launcher SHALL be 720 logical pixels wide. The SearchBar SHALL be 56
logical pixels high, the ActionBar 40, and a list row 40.

#### Scenario: Root search

- **WHEN** the launcher opens at root
- **THEN** it shows the SearchBar, a list of results, and the ActionBar

#### Scenario: Sizes at default text scale

- **WHEN** root search is shown at the default text scale
- **THEN** the launcher is 720 logical pixels wide, the SearchBar is 56 high,
  each result row is 40 high, and the ActionBar is 40 high

### Requirement: The SearchBar holds the query and shows where the user is

The SearchBar SHALL hold the query for a list or grid, with a placeholder the
view sets. When a view has been pushed, it SHALL show a back control before
the query; activating it SHALL go back one view, as Escape does. While the view
is loading, it SHALL show a thin static bar along its bottom edge in the accent
role. When the view declares a filter dropdown, the SearchBar SHALL show it at
its right end. For a detail or form view, the SearchBar SHALL show the
command's title in place of the query.

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
- **THEN** the loading bar is shown under the SearchBar and does not move

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
style and one line of text. A busy toast SHALL show a muted dot and stay while
its work runs. A success toast SHALL show an accent dot and disappear after
about two seconds. A failure toast SHALL show a danger dot and stay until the
next key press or click. The dot SHALL NOT move. Assistive technology SHALL be
told the toast's text and whether it reports a failure. Showing a toast SHALL
NOT take focus.

#### Scenario: A success toast

- **WHEN** an action that keeps the launcher open reports success with a toast
- **THEN** the toast shows with an accent dot and is gone after about two
  seconds, and the context returns

#### Scenario: A failure toast stays

- **WHEN** an action fails
- **THEN** a failure toast shows its sentence with a danger dot
- **AND** it stays until the user presses a key or clicks

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
- A tag is short text in a rounded outline coloured by the accent, muted, or
  danger role.
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

### Requirement: Each view kind has one layout

- A list SHALL show its rows under optional section headings. A heading shows
  its title and, when given, its subtitle such as a count. With a detail pane,
  the list SHALL take two fifths of the width and the detail of the selected
  item the rest.
- A grid SHALL show tiles in a fixed number of columns at the declared aspect
  ratio, each with its title below, under optional section headings. The
  selected tile SHALL be outlined in the accent role.
- A detail SHALL show its body and, when declared, its metadata as label and
  value pairs below the body, labels in the muted role.
- A form SHALL show a label column and a control column. A field's error SHALL
  be shown under the field in the danger role and be its accessible
  description.
- An empty view SHALL show one icon, one title, and at most one line of hint,
  centred in the view.

#### Scenario: A list with a detail pane

- **WHEN** a list declares a detail pane and a row is selected
- **THEN** the list fills the left two fifths and the selected item's detail the
  right three fifths

#### Scenario: A form error

- **WHEN** a form field reports an error
- **THEN** its message shows under that field in the danger role and is exposed
  as the field's description, and the field can still be edited

#### Scenario: No results

- **WHEN** a query matches nothing
- **THEN** the empty view shows its icon, "No results for" followed by the
  query, and its hint when one is declared

### Requirement: Selection, hover, and focus are told apart

The selected row, tile, or action SHALL be filled with the selection role. A
hovered one SHALL be filled with the hover role, which SHALL be clearly weaker
than selection, following the list-navigation requirements. A focused form
control SHALL show a border in the accent role. A surface that keeps focus in
its input and moves a selection SHALL show that selection, not a focus ring
around the surface.

#### Scenario: Hovering another row

- **WHEN** a row is selected and the pointer rests on another
- **THEN** the hovered row is clearly weaker than the selected row

#### Scenario: Tabbing through a form

- **WHEN** the user presses Tab in a form
- **THEN** the next control shows the accent border and the previous one loses
  it

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

Dango SHALL NOT animate its interface. Showing the launcher, moving the
selection, hovering, opening and closing the action panel, showing and removing
a toast, changing results, and replacing a view SHALL each change in one frame.
Busy states SHALL be shown by static marks next to text.

#### Scenario: Activation on Windows

- **WHEN** the launcher is shown on Windows, first or again
- **THEN** it is painted without visible delay and without an entrance effect

#### Scenario: Activation on macOS

- **WHEN** the launcher is shown on macOS, first or again
- **THEN** it is painted without visible delay and without an entrance effect

#### Scenario: A streamed view stays stable

- **WHEN** a command emits thirty replacements of one view over one second
- **THEN** the content updates without flicker, and selection and focus stay

#### Scenario: Busy without motion

- **WHEN** work is in progress
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
