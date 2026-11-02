# Spec Delta

## MODIFIED Requirements

### Requirement: The keystroke fallback can be refused for an application

The user SHALL be able to name applications where the clipboard fallback must
not be attempted, because the copy chord does something else there. For a
named application Dango SHALL report that it cannot read the selection, and
SHALL send no keystroke.

The applications SHALL be set by the config key
`selection.excluded-applications`: a list of application names, matched
ignoring case, the same way as the clipboard history's exclusion list. It SHALL
be empty by default.

#### Scenario: A named application is never sent the copy chord

- **WHEN** the frontmost application is one the user named, and something reads
  the selection
- **THEN** no keystroke is sent to it
- **AND** the failure says to copy the text first, as that application does not
  share its selection

#### Scenario: Names match ignoring case

- **WHEN** `selection.excluded-applications` names "helix" and the frontmost
  application is named "Helix"
- **THEN** it is treated as a named application

#### Scenario: A named application that does expose its selection still works

- **WHEN** a named application exposes its selection directly
- **THEN** the selection is read directly
- **AND** the exclusion never comes into play

#### Scenario: An application that is not named is unaffected

- **WHEN** the frontmost application is not named and does not expose its
  selection
- **THEN** the clipboard fallback runs

#### Scenario: The list is empty by default

- **WHEN** `selection.excluded-applications` is not set
- **THEN** every application that does not expose its selection gets the
  clipboard fallback

