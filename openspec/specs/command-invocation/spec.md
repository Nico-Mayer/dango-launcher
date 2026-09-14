# command-invocation Specification

## Purpose

How a command declared by an extension is actually run: who executes it, what a
no-view command reports when it finishes, how a view command puts its first
screen up, and what the user sees when a command fails, hangs, or is invoked
while another is still working.

## Requirements

### Requirement: A command is resolved by its qualified identity

An invocation SHALL name the command by its fully qualified identity, so that
two extensions declaring the same command identifier never invoke each other's.

#### Scenario: Two extensions declare the same command identifier

- **WHEN** a command is invoked and another enabled extension declares a command with the same unqualified identifier
- **THEN** the command belonging to the named extension is the one invoked

#### Scenario: Command no longer exists

- **WHEN** the user confirms a command whose extension was disabled since the results were produced
- **THEN** nothing is invoked
- **AND** the launcher stays open and reports that the command is unavailable

### Requirement: Every command names the host that runs it

The registry SHALL record which host executes each command, and invocation SHALL
go through that host rather than calling the extension directly. Only the
built-in native host exists in this change.

#### Scenario: Built-in command runs on the native host

- **WHEN** a command contributed by a built-in extension is invoked
- **THEN** it is executed by the built-in native host

#### Scenario: Host is unavailable

- **WHEN** a command names a host that this build cannot run
- **THEN** the command is not invoked
- **AND** the user is told the command cannot run in this build

### Requirement: A no-view command reports its outcome

A command declared `no-view` SHALL perform its work and report either success or
a failure with a message. It SHALL NOT put a view on screen.

#### Scenario: No-view command succeeds

- **WHEN** a no-view command completes successfully
- **THEN** the launcher hides
- **AND** no view is pushed

#### Scenario: No-view command fails

- **WHEN** a no-view command fails
- **THEN** the launcher stays open
- **AND** the failure message is shown to the user

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

### Requirement: A slow command does not freeze the launcher

Invocation SHALL NOT block the launcher. A command that takes time to produce
its first view SHALL leave the launcher responsive, and the user SHALL be able
to abandon it.

#### Scenario: Command is slow to produce a view

- **WHEN** a view command has been invoked but has not yet delivered a tree
- **THEN** the launcher remains responsive to input
- **AND** a loading state is shown

#### Scenario: User abandons a slow command

- **WHEN** the user dismisses the launcher while a command is still working
- **THEN** the launcher hides
- **AND** the abandoned command's later output is discarded rather than shown

### Requirement: One invocation at a time

Invoking a command while another is running SHALL supersede the first. The
launcher SHALL never show output from a superseded invocation.

#### Scenario: A second command supersedes the first

- **WHEN** the user invokes a command while an earlier one is still running
- **THEN** the earlier invocation's output is discarded
- **AND** only the newer command's output is shown

### Requirement: An extension owns the results it contributes

Acting on a result SHALL be dispatched to the extension that contributed it. No
extension SHALL be privileged as the default handler.

#### Scenario: Action is dispatched to the contributing extension

- **WHEN** the user chooses an action on a result contributed by an extension
- **THEN** that extension performs the action

#### Scenario: Two extensions contribute results in one list

- **WHEN** results from two different extensions appear in the same list and the user acts on each
- **THEN** each action is performed by the extension that contributed its result
