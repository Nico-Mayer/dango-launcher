## MODIFIED Requirements

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
