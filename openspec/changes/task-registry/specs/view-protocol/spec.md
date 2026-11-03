# Spec Delta

## MODIFIED Requirements

### Requirement: View kinds

The protocol SHALL be at version 2 and SHALL support five view kinds: a list, a
grid, a detail view rendering markdown, a form, and a progress view. A progress
view SHALL declare a title and MAY declare a subtitle, a progress value from 0
to 1, a one-line detail, and steps, each with a title and a status of working,
needs input, done, failed, or queued. A progress view without a value SHALL show
an indeterminate bar, and a value outside 0 to 1 SHALL be shown clamped. Any
view kind SHALL be able to carry an action panel. A view tree at version 1
SHALL be rejected as an unsupported version.

#### Scenario: List renders items

- **WHEN** a command emits a list view containing items
- **THEN** each item's title, and its subtitle, icon, and accessories when
  present, are rendered in order

#### Scenario: Grid renders tiles

- **WHEN** a command emits a grid view containing items
- **THEN** each item is rendered as a tile with its content and title, in order,
  in the declared number of columns

#### Scenario: Detail renders markdown

- **WHEN** a command emits a detail view
- **THEN** its markdown content is rendered

#### Scenario: Form collects input

- **WHEN** a command emits a form view and the user submits it
- **THEN** the entered values are delivered to the command

#### Scenario: Progress view shows steps

- **WHEN** a command emits a progress view with a value of 0.62 and four steps
- **THEN** its bar is filled to 62 percent and its steps are listed in order
  with their status

#### Scenario: A version 1 tree

- **WHEN** a command emits a view tree at protocol version 1
- **THEN** an error is shown in place of the view
