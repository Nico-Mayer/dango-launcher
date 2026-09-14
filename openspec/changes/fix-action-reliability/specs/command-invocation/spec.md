## MODIFIED Requirements

### Requirement: A view command delivers a view tree

A command declared `view` SHALL deliver a view tree carrying the protocol
version. The launcher SHALL push that tree onto its view stack and render it.

Each delivered tree SHALL be accompanied by the extension that owns the command
that produced it, so that an action chosen in that view reaches its command
however the command was started. The launcher SHALL NOT depend on how the
command was invoked to learn who owns the view.

#### Scenario: View command renders its first screen

- **WHEN** a view command is invoked
- **THEN** a view tree is delivered and pushed onto the view stack
- **AND** the launcher stays open showing it

#### Scenario: View command replaces its own screen

- **WHEN** an already-running view command delivers a further view tree
- **THEN** the tree on top of the stack is replaced whole
- **AND** no partial update is applied

#### Scenario: An action reaches the command that produced the view

- **WHEN** the user chooses an action in a pushed view
- **THEN** it is performed by the extension that owns the command that delivered
  that view

#### Scenario: A view the launcher did not ask for still accepts actions

- **WHEN** a view command is started without the launcher having requested it,
  as a command's own hotkey does, and the user chooses an action in the view
- **THEN** the action is performed, exactly as it would be for a view opened
  from root search

#### Scenario: View tree carries an unsupported protocol version

- **WHEN** a delivered view tree declares a protocol version the launcher does not support
- **THEN** the tree is not rendered
- **AND** a dismissible error is shown
