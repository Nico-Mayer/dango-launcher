## MODIFIED Requirements

### Requirement: Versioned protocol

The view protocol SHALL carry a `protocolVersion`. Dango SHALL reject a view
tree whose version it does not support and SHALL show an error rather than
rendering partially. The check SHALL happen before the tree reaches the
renderer, so every renderer shows the same error for the same tree.

#### Scenario: Unsupported protocol version

- **WHEN** a command emits a view tree with an unsupported `protocolVersion`
- **THEN** an error is shown in place of the view
- **AND** the user can dismiss it and return to root

### Requirement: Full-tree replace

A command SHALL communicate a change by emitting a complete new view tree. The
protocol SHALL NOT define partial updates, patches, or diffs. Dango SHALL
identify list items by their identifier so that replacing a tree does not lose
selection or cause visible flicker.

#### Scenario: Selection survives a tree replacement

- **WHEN** the user has selected the third item and the command emits a new tree containing that same item identifier
- **THEN** the item remains selected

#### Scenario: Streaming content replaces the tree repeatedly

- **WHEN** a command emits thirty view trees over one second
- **THEN** each renders without flicker and without dropped frames
