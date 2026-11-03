# Spec Delta

## MODIFIED Requirements

### Requirement: Expansion is skipped where it should not happen

Dango SHALL hold nothing and expand nothing in contexts the user has excluded,
the system marks as sensitive, or that belong to Dango itself. The excluded
applications are the clipboard extension's `excluded-applications` setting,
at `clipboard.excluded-applications`: one list
serves both clipboard history and keyword expansion.

On Windows, a password field is recognised only when it is a classic Win32
edit control marked as a password field. A password field in a browser or in
another custom control is not recognised, so a keyword typed there SHALL be
expected to expand.

#### Scenario: A password field on macOS

- **WHEN** the system reports that secure input is active
- **THEN** nothing is observed, nothing is held, and nothing expands

#### Scenario: A password field on Windows

- **WHEN** the user types into a classic Win32 edit control marked as a password field
- **THEN** nothing is held and nothing expands

#### Scenario: A browser password field on Windows

- **WHEN** the user types a keyword into a password field in a browser or another custom control
- **THEN** the keyword expands, because Windows offers no reliable way to know it is a password field

#### Scenario: An excluded application

- **WHEN** the user types in an application listed in the clipboard extension's `excluded-applications` setting
- **THEN** nothing is held and nothing expands

#### Scenario: The exclusion list is honoured without a restart

- **WHEN** the user changes the `excluded-applications` setting
- **THEN** the change takes effect on the next keystroke observed

#### Scenario: Typing in the launcher on Windows

- **WHEN** on Windows the user types a snippet's keyword into the launcher's search field
- **THEN** nothing expands and nothing is held
- **AND** the search field shows exactly what the user typed

#### Scenario: Typing in the launcher on macOS

- **WHEN** on macOS the user types a snippet's keyword into the launcher's search field, while the previous app is still the frontmost application
- **THEN** nothing expands and nothing is held
- **AND** the search field shows exactly what the user typed

