## MODIFIED Requirements

### Requirement: A template renders its placeholders

A template SHALL be text in Jinja syntax. A placeholder SHALL be a name between
double braces, as in `{{ name }}`. `{% ... %}` SHALL mark a statement and
`{# ... #}` a comment, filters such as `{{ name | upper }}` SHALL apply to a
value, and text between `{% raw %}` and `{% endraw %}` SHALL be kept as
written. Rendering SHALL replace each placeholder with its value and leave all
text outside these delimiters exactly as written.

#### Scenario: A placeholder is replaced

- **WHEN** a template containing a placeholder is rendered with a value for it
- **THEN** the result contains that value in place of the placeholder

#### Scenario: Text around placeholders is untouched

- **WHEN** a template containing no placeholders is rendered
- **THEN** the result is identical to the template

#### Scenario: Braces that are not placeholders are left alone

- **WHEN** a template contains single braces, such as a block of code
- **THEN** they appear unchanged in the result

#### Scenario: A filter changes a value

- **WHEN** a template containing `{{ name | upper }}` is rendered with `name`
  set to "dango"
- **THEN** the result contains "DANGO"

#### Scenario: Double braces can be kept as text

- **WHEN** a template contains `{% raw %}${{ matrix.os }}{% endraw %}`
- **THEN** the result contains `${{ matrix.os }}`
- **AND** no argument is reported for it

#### Scenario: Rendering is fast enough to be invisible

- **WHEN** a template of 1,000 characters containing 10 placeholders is rendered
- **THEN** the result is produced without visible delay

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
- **THEN** the current local date appears in its place, written as `YYYY-MM-DD`

#### Scenario: The uuid placeholder

- **WHEN** a template using `uuid` is rendered twice
- **THEN** a different identifier appears each time

#### Scenario: A reserved name is never asked for

- **WHEN** a template uses only reserved names
- **THEN** the user is asked for nothing and the template renders immediately

#### Scenario: A reserved value that is not available

- **WHEN** a template uses `clipboard` or `selection` and there is none
- **THEN** it renders as empty text rather than failing
