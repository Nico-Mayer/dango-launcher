## ADDED Requirements

### Requirement: A result carries its extension's tint

A root search result SHALL be presented with the tint declared by the extension
that contributed it, so results from different extensions can be told apart
without reading their titles. The tint SHALL be applied as a filled background
behind the result's icon, with the icon drawn in a colour that stays legible
against it in both the light and the dark theme.

The tint SHALL apply only where the icon is one of the launcher's own named
icons. A result whose icon is an image the extension produced, such as an
application's own icon, SHALL be presented without a tinted background, and so
SHALL a result with no icon at all.

The tint SHALL NOT change which results appear, their order, or the time taken
to produce them.

#### Scenario: Two built-in commands are told apart

- **WHEN** a query returns commands from two extensions that declare different tints
- **THEN** each command's icon sits on its own extension's tint

#### Scenario: Commands of one extension share its tint

- **WHEN** a query returns several commands from the same extension
- **THEN** every one of them shows the same tint

#### Scenario: An application result keeps its own icon

- **WHEN** a query returns an application whose own icon was extracted
- **THEN** that icon is shown with no tinted background behind it

#### Scenario: An untinted extension is unchanged

- **WHEN** a query returns a result from an extension that declares no tint
- **THEN** the result is presented as it was before tints existed

#### Scenario: A dynamic result is tinted like its extension's commands

- **WHEN** a query returns an item from a root items provider whose extension declares a tint, and the item's icon is a named icon
- **THEN** it shows the same tint as that extension's commands

#### Scenario: Tinting does not cost search time

- **WHEN** the index holds 2000 candidates and the user types a character
- **THEN** ranked results are still ready within 30 milliseconds
