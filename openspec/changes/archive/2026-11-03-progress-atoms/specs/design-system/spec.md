# Spec Delta

## ADDED Requirements

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

The busy marks, the SearchBar's loading bar and the busy toast, SHALL appear
only once Dango has been busy for about 150 milliseconds. Dango is busy while a
command invoked from root search has not finished or shown a view, or while
the top view is marked loading. Once shown, the marks SHALL stay for at least
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

## MODIFIED Requirements

### Requirement: The SearchBar holds the query and shows where the user is

The SearchBar SHALL hold the query for a list or grid, with a placeholder the
view sets. When a view has been pushed, it SHALL show a back control before
the query; activating it SHALL go back one view, as Escape does. While the busy
marks are shown, it SHALL show an indeterminate progress bar along its bottom
edge: a thin bar in which a short segment in the accent role sweeps from one
end to the other, again and again, at a steady pace. When the view declares a
filter dropdown, the SearchBar SHALL show it at its right end. For a detail or
form view, the SearchBar SHALL show the command's title in place of the query.

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

- **WHEN** the busy marks are shown for a view marked loading
- **THEN** the loading bar is shown under the SearchBar with its segment
  sweeping along it

#### Scenario: Loading ends

- **WHEN** the busy marks are removed, or the user leaves the view
- **THEN** the loading bar and its motion are gone in the same frame

#### Scenario: A form shows its title

- **WHEN** a command pushes a form
- **THEN** the SearchBar shows that command's title and accepts no typing

### Requirement: A toast reports status in the ActionBar

A toast SHALL replace the ActionBar's context while it shows. A toast has a
style and one line of text. A busy toast SHALL show a spinner in the working
role and stay while its work runs, following the busy-mark timing. A success
toast SHALL show a dot in the done role and disappear after about two seconds.
A failure toast SHALL show a dot in the danger role and stay until the next key
press or click. The success and failure dots SHALL NOT move. Assistive
technology SHALL be told the toast's text and whether it reports a failure.
Showing a toast SHALL NOT take focus.

#### Scenario: A success toast

- **WHEN** an action that keeps the launcher open reports success with a toast
- **THEN** the toast shows with a dot in the done role and is gone after about
  two seconds, and the context returns

#### Scenario: A failure toast stays

- **WHEN** an action fails
- **THEN** a failure toast shows its sentence with a dot in the danger role
- **AND** it stays until the user presses a key or clicks

#### Scenario: A busy toast

- **WHEN** the busy marks are shown
- **THEN** the busy toast shows a spinner in the working role, turning while
  the work runs

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
- A progress ring as an accessory is 16 logical pixels.
- An accessory is text, an icon, a tag, a key chord, a progress ring, or a
  status dot. A row shows at most three, right-aligned.

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

## REMOVED Requirements

### Requirement: Nothing animates

**Reason**: Too strict. A still busy mark looks the same as an idle one, and
the rule saved next to nothing while idle. Motion is now allowed where it
shows that something is busy: progress marks and the busy toast's spinner.
**Migration**: None. The launcher still shows, moves its selection, opens the
action panel, and replaces views without transitions, because no requirement
asks for one. That a streamed view keeps its loading bar steady moves to
"Busy marks do not blink". How fast the launcher appears stays in
launcher-shell, and how a streamed view keeps its selection stays in
view-protocol.
