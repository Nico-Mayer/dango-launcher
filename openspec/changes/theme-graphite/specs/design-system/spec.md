# Spec Delta

## ADDED Requirements

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

## MODIFIED Requirements

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
