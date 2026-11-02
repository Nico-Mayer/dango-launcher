# Spec Delta

## ADDED Requirements

### Requirement: Each result names its extension

Each root search result SHALL show, beside its title, the name of the
extension that owns it, as that extension's manifest gives it. This SHALL hold
for commands and for root items alike. A pinned result SHALL show its own
status line there instead. No other result SHALL show a path, a content
preview, or any other text in that place.

#### Scenario: An application names its extension

- **WHEN** root search lists an application
- **THEN** the text beside its title reads "Applications"
- **AND** the application's path is not shown in the row

#### Scenario: A command names its extension

- **WHEN** root search lists Clipboard History
- **THEN** the text beside its title reads "Clipboard History"

#### Scenario: A root item from a provider names its extension

- **WHEN** root search lists a Steam game
- **THEN** the text beside its title reads "Games"

#### Scenario: A snippet shows no preview

- **WHEN** root search lists a snippet
- **THEN** the text beside its title reads "Snippets"
- **AND** no part of the snippet's body is shown in the row

#### Scenario: A pinned result keeps its status line

- **WHEN** a missing permission is listed as a pinned result
- **THEN** the text beside its title is that permission's status line, not "Permissions"
