## MODIFIED Requirements

### Requirement: Nothing animates

Dango SHALL NOT animate its interface. Showing the launcher, moving the
selection, hovering, opening and closing the action panel, showing and removing
a toast, changing results, and replacing a view SHALL each change in one frame.
Busy states SHALL be shown by static marks next to text. How fast the launcher
appears is in launcher-shell, and how a streamed view keeps its selection is in
view-protocol.

#### Scenario: Activation on Windows

- **WHEN** the launcher is shown on Windows, first or again
- **THEN** it appears without an entrance effect

#### Scenario: Activation on macOS

- **WHEN** the launcher is shown on macOS, first or again
- **THEN** it appears without an entrance effect

#### Scenario: A streamed view stays stable

- **WHEN** a command replaces one view many times a second
- **THEN** each replacement shows in one frame, with no transition between the
  old and the new content

#### Scenario: Busy without motion

- **WHEN** work is in progress
- **THEN** no animation runs on any surface

## REMOVED Requirements

### Requirement: Every surface takes its look from one theme

**Reason**: It states how the code is built, not behaviour seen from outside.
AGENTS.md already holds the rule that every
colour, radius, size, and type face comes from the `Theme` global.

**Migration**: None. The rule lives in AGENTS.md under the tech stack.
