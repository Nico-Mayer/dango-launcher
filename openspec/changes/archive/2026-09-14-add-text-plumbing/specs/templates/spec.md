## Purpose

The one place Dango decides what a placeholder is: which names fill themselves
in, which names become questions asked of the user, and what a template turns
into once both are resolved. Shared by every feature that stores a string with
holes in it.

## ADDED Requirements

### Requirement: A template renders its placeholders

A template SHALL be plain text containing placeholders written as a name between
double braces. Rendering SHALL replace each placeholder with its value and leave
all other text exactly as written.

#### Scenario: A placeholder is replaced

- **WHEN** a template containing a placeholder is rendered with a value for it
- **THEN** the result contains that value in place of the placeholder

#### Scenario: Text around placeholders is untouched

- **WHEN** a template containing no placeholders is rendered
- **THEN** the result is identical to the template

#### Scenario: Braces that are not placeholders are left alone

- **WHEN** a template contains single braces, such as a block of code
- **THEN** they appear unchanged in the result

#### Scenario: Rendering is fast enough to be invisible

- **WHEN** a template of 1,000 characters containing 10 placeholders is rendered
- **THEN** the result is produced in under 5ms

### Requirement: A fixed vocabulary fills itself in

The names `clipboard`, `selection`, `date`, `uuid`, `cursor`, and `query` SHALL
be reserved. A reserved name SHALL resolve without asking the user for anything,
except `query`, which is supplied by the caller.

#### Scenario: The clipboard placeholder

- **WHEN** a template using `clipboard` is rendered
- **THEN** the current clipboard text appears in its place

#### Scenario: The selection placeholder

- **WHEN** a template using `selection` is rendered
- **THEN** the text selected in the frontmost application appears in its place

#### Scenario: The date placeholder

- **WHEN** a template using `date` is rendered
- **THEN** the current date appears in its place

#### Scenario: The uuid placeholder

- **WHEN** a template using `uuid` is rendered twice
- **THEN** a different identifier appears each time

#### Scenario: A reserved name is never asked for

- **WHEN** a template uses only reserved names
- **THEN** the user is asked for nothing and the template renders immediately

#### Scenario: A reserved value that is not available

- **WHEN** a template uses `clipboard` or `selection` and there is none
- **THEN** it renders as empty text rather than failing

### Requirement: Any other name is an argument

A placeholder whose name is not reserved SHALL be an argument: a value the user
is asked for before the template can render. The arguments a template needs
SHALL be reported before it is rendered, in the order they first appear in it.

#### Scenario: Arguments are reported before rendering

- **WHEN** a template using two names that are not reserved is inspected
- **THEN** both names are reported as arguments

#### Scenario: Argument order follows the template

- **WHEN** a template's arguments are reported
- **THEN** they are in the order the names first appear in the template

#### Scenario: A repeated argument is asked for once

- **WHEN** a template uses the same argument name twice
- **THEN** it is reported once
- **AND** rendering puts the same value in both places

#### Scenario: A template with no arguments reports none

- **WHEN** a template using only reserved names is inspected
- **THEN** no arguments are reported

#### Scenario: An argument left empty

- **WHEN** a template is rendered with an argument the user left blank
- **THEN** it renders as empty text and the rest of the template is unaffected

### Requirement: A query is made safe for where it is going

When a rendered template is used as a URL, a value placed into it SHALL be
encoded so that spaces and reserved characters cannot change the URL's meaning.

#### Scenario: A query with spaces

- **WHEN** a URL template's query is rendered with a value containing spaces
- **THEN** the spaces are encoded and the resulting URL is valid

#### Scenario: A query with characters that have meaning in a URL

- **WHEN** a query value contains characters such as `&`, `?`, or `#`
- **THEN** they are encoded rather than treated as part of the URL's structure

### Requirement: A caret position can be declared

A template MAY use `cursor` to declare where the caret should end up. It SHALL
contribute no characters to the rendered text.

#### Scenario: The cursor placeholder leaves no text

- **WHEN** a template using `cursor` is rendered
- **THEN** the result contains no marker of any kind
- **AND** the position it declared is reported alongside the text

#### Scenario: A template with no cursor placeholder

- **WHEN** a template that does not use `cursor` is rendered
- **THEN** no caret position is reported

#### Scenario: More than one cursor placeholder

- **WHEN** a template uses `cursor` more than once
- **THEN** the first is honoured
- **AND** the others contribute nothing

### Requirement: A template that cannot be read is rejected where it is written

A template SHALL be checked when the user saves it, and a template that cannot
be parsed SHALL be refused with a message, rather than failing later when it is
used.

#### Scenario: Saving a template that cannot be parsed

- **WHEN** the user saves a template with an unclosed placeholder
- **THEN** it is refused with a message explaining what is wrong
- **AND** nothing is stored

#### Scenario: A stored template that no longer parses

- **WHEN** a stored template cannot be parsed at the moment it is used
- **THEN** the failure is reported to the user
- **AND** nothing is inserted into any application
