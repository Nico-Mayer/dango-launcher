# Spec Delta

## MODIFIED Requirements

### Requirement: Every screen is one shell around one view

The launcher SHALL show, from top to bottom, the SearchBar, exactly one view,
and the ActionBar. The view is a list, a grid, a detail, a form, a progress
view, or an empty view. In root search, an answer card MAY sit between the
SearchBar and the results; when no result matches, the card alone stands in
for the empty view. Overlays SHALL float above these regions and SHALL NOT
move them.

The launcher SHALL be 720 logical pixels wide. The SearchBar SHALL be 64
logical pixels high, the ActionBar 44, and a list row 44.

#### Scenario: Root search

- **WHEN** the launcher opens at root
- **THEN** it shows the SearchBar, a list of results, and the ActionBar

#### Scenario: Sizes at default text scale

- **WHEN** root search is shown at the default text scale
- **THEN** the launcher is 720 logical pixels wide, the SearchBar is 64 high,
  each result row is 44 high, and the ActionBar is 44 high

#### Scenario: Root search with an answer

- **WHEN** the user types "2+2" at root
- **THEN** the launcher shows the SearchBar, the answer card, the results, and
  the ActionBar, from top to bottom

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

### Requirement: A toast reports status in the ActionBar

A toast SHALL replace the ActionBar's context while it shows. A toast has a
style and one line of text, and MAY carry one action, shown after the text as
its label and chord. Pressing that chord while the toast shows SHALL run the
action and SHALL NOT also act on the view. A busy toast SHALL show a spinner in
the working role and stay while its work runs, following the busy-mark timing.
A success toast SHALL show a dot in the done role and disappear after about two
seconds. A failure toast SHALL show a dot in the danger role and stay until the
next key press or click. The success and failure dots SHALL NOT move. Assistive
technology SHALL be told the toast's text, its action, and whether it reports
a failure. Showing a toast SHALL NOT take focus.

#### Scenario: A success toast

- **WHEN** an action that keeps the launcher open reports success with a toast
- **THEN** the toast shows with a dot in the done role and is gone after about
  two seconds, and the context returns

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

### Requirement: Busy marks do not blink

The busy marks, the SearchBar's loading bar and the busy toast, SHALL appear
only once Dango has been busy for about 150 milliseconds. Dango is busy while a
command invoked from root search has not finished or shown a view, while the
top view is marked loading, or while a background task runs and its progress
view is not shown. A running background task SHALL show only the busy toast,
never the loading bar. Once shown, the marks SHALL stay for at least
about 400 milliseconds, even if the work ends sooner. While Dango stays busy,
replacing the view SHALL NOT restart the marks. Leaving the view or a reset
SHALL remove them at once.

#### Scenario: A fast command

- **WHEN** a command invoked from root search finishes within about 150
  milliseconds
- **THEN** no busy toast is shown

#### Scenario: A slow command shows a view that loads

- **WHEN** a command takes a second to show a view marked loading, and the
  view loads for another second
- **THEN** the busy toast appears after a short moment reading "Running"
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
- **THEN** the busy toast shows the download and the SearchBar shows no
  loading bar
