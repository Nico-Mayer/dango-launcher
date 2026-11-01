## ADDED Requirements

### Requirement: A detail can show an image

A detail, standalone or in a list's pane, SHALL be able to declare an image,
given as a named icon or a path to an image file. The image SHALL be shown
above the body, scaled down to fit the detail's width with its aspect ratio
kept, and never scaled up. A detail without an image SHALL show only its body.

#### Scenario: Image above the body

- **WHEN** a detail declares an image and a body
- **THEN** the image is shown above the body

#### Scenario: A large image fits

- **WHEN** a detail declares an image wider than the detail
- **THEN** the image is scaled down to the detail's width, keeping its aspect ratio

#### Scenario: A small image is not enlarged

- **WHEN** a detail declares an image narrower than the detail
- **THEN** the image is shown at its own size

#### Scenario: An image that cannot be loaded

- **WHEN** a detail declares an image path that does not exist
- **THEN** the body and metadata are still shown, without an image

### Requirement: A detail can show plain text

A detail, standalone or in a list's pane, SHALL be able to declare plain text
as its body instead of markdown. Plain text SHALL be shown exactly as given,
with its line breaks and spaces, and SHALL NOT be read as markdown. When a
detail declares both, the plain text SHALL be shown.

#### Scenario: Plain text is not formatted

- **WHEN** a detail declares the plain text `# not a heading`
- **THEN** the detail shows `# not a heading` as written

#### Scenario: Plain text keeps its layout

- **WHEN** a detail declares plain text with line breaks and leading spaces
- **THEN** the detail shows the same line breaks and leading spaces

#### Scenario: Plain text wins over markdown

- **WHEN** a detail declares both markdown and plain text
- **THEN** only the plain text is shown
