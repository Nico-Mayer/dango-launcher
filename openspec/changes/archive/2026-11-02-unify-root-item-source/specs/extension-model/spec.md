# Spec Delta

## MODIFIED Requirements

### Requirement: Command declaration

Each declared command SHALL carry an identifier unique within its extension, a
title, an invocation mode of either `view` or `no-view`, and optionally an
icon, search keywords, and a user-assigned alias. Its fully qualified identity
SHALL be the extension identifier combined with the command identifier.

#### Scenario: Command is addressable

- **WHEN** an extension declaring a command is loaded
- **THEN** that command can be resolved by its fully qualified identity

#### Scenario: Keywords widen matching

- **WHEN** a command declares a keyword that does not appear in its title
- **AND** the user types that keyword
- **THEN** the command appears in results
