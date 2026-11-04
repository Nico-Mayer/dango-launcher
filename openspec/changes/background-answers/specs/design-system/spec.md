# Spec Delta

## MODIFIED Requirements

### Requirement: A toast reports status in the ActionBar

A toast SHALL replace the ActionBar's context while it shows. A toast has a
style and one line of text, and MAY carry one action, shown after the text as
its label and chord. Pressing that chord while the toast shows SHALL run the
action and SHALL NOT also act on the view. A busy toast SHALL show a spinner in
the working role and stay while its work runs, following the busy-mark timing.
A success toast SHALL show a dot in the done role and disappear after about two
seconds, unless it carries an action: then it SHALL stay until the next key
press or click, so the action is still there when the user looks. A failure
toast SHALL show a dot in the danger role and stay until the next key press or
click. The success and failure dots SHALL NOT move. Assistive
technology SHALL be told the toast's text, its action, and whether it reports
a failure. Showing a toast SHALL NOT take focus.

#### Scenario: A success toast

- **WHEN** an action that keeps the launcher open reports success with a toast
- **THEN** the toast shows with a dot in the done role and is gone after about
  two seconds, and the context returns

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

### Requirement: Small parts look the same everywhere

The launcher SHALL build every surface from the same small parts:

- An icon is 20 logical pixels, square or round. A named icon is drawn in a
  theme role. An application icon or favicon keeps its own colours and aspect
  ratio. A content preview, such as a copied image, fills its frame and has a
  border.
- A key chord is drawn as keys in Geist Mono, using the platform's names as the
  view protocol requires, in a chip that fits its text. On Windows the keys are
  separated by a space; on macOS they are joined with none. Every key chord
  uses this one format: the footer, the action panel, toasts, and result rows.
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

- **WHEN** the launcher shows the action panel's chord, a toast's "Show answer"
  chord, and a result's hotkey on Windows
- **THEN** they read "Ctrl K", "Ctrl ↵", and "Ctrl Alt V"

#### Scenario: Key chords on macOS

- **WHEN** the launcher shows the action panel's chord, a toast's "Show answer"
  chord, and a result's hotkey on macOS
- **THEN** they read "⌘K", "⌘↵", and "⌃⌥V"

#### Scenario: A fourth accessory

- **WHEN** an item declares four accessories
- **THEN** the row shows the first three

#### Scenario: A tag in the accent role

- **WHEN** an item declares the tag "Link" in the accent role
- **THEN** "Link" is drawn in the accent colour on a faint accent fill, with no
  outline
