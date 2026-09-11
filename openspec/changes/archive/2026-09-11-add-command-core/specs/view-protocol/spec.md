## Purpose

The versioned declarative contract describing what a command puts on screen and
which actions it offers, so that the frontend renders any command, built-in or
otherwise, without knowing anything about it.

## ADDED Requirements

### Requirement: Versioned protocol

The view protocol SHALL carry a `protocolVersion`. The frontend SHALL reject a
view tree whose version it does not support and SHALL show an error rather than
rendering partially.

#### Scenario: Unsupported protocol version

- **WHEN** a command emits a view tree with an unsupported `protocolVersion`
- **THEN** an error is shown in place of the view
- **AND** the user can dismiss it and return to root

### Requirement: View kinds

The protocol SHALL support three view kinds at version 1: a list, a detail view
rendering markdown, and a form. Any view kind SHALL be able to carry an action
panel.

#### Scenario: List renders items

- **WHEN** a command emits a list view containing items
- **THEN** each item's title, and its subtitle and icon when present, are rendered in order

#### Scenario: Detail renders markdown

- **WHEN** a command emits a detail view
- **THEN** its markdown content is rendered

#### Scenario: Form collects input

- **WHEN** a command emits a form view and the user submits it
- **THEN** the entered values are delivered to the command

### Requirement: Full-tree replace

A command SHALL communicate a change by emitting a complete new view tree. The
protocol SHALL NOT define partial updates, patches, or diffs. The frontend SHALL
identify list items by their identifier so that replacing a tree does not lose
selection or cause visible flicker.

#### Scenario: Selection survives a tree replacement

- **WHEN** the user has selected the third item and the command emits a new tree containing that same item identifier
- **THEN** the item remains selected

#### Scenario: Streaming content replaces the tree repeatedly

- **WHEN** a command emits thirty view trees over one second
- **THEN** each renders without flicker and without dropped frames

### Requirement: Action panel

Any view SHALL be able to declare a list of actions, the first of which is the
primary action. The primary action SHALL be invocable with Enter and the panel
SHALL be openable to reach the remaining actions. Each action MAY declare its
own keyboard shortcut.

#### Scenario: Primary action runs on Enter

- **WHEN** a view declares actions and the user presses Enter
- **THEN** the first action is invoked

#### Scenario: Secondary action is reachable

- **WHEN** the user opens the action panel and chooses a secondary action
- **THEN** that action is invoked

#### Scenario: View declares no actions

- **WHEN** a view declares no actions and the user presses Enter
- **THEN** nothing is invoked and the view is unchanged

### Requirement: Filtering ownership is declared

A list view SHALL declare whether the launcher filters its items against the
query or the command handles filtering itself. When the launcher filters, the
command SHALL NOT receive keystrokes as query changes.

#### Scenario: Launcher filters by default

- **WHEN** a list declares that the launcher filters and the user types
- **THEN** the visible items are narrowed without the command being invoked again

#### Scenario: Command owns filtering

- **WHEN** a list declares that the command filters and the user types
- **THEN** the command receives the updated query and may emit a new tree

### Requirement: Loading and empty states

A view SHALL be able to signal that it is loading and to declare what is shown
when it has no content.

#### Scenario: Loading is visible

- **WHEN** a command emits a view marked as loading
- **THEN** a loading indicator is shown without clearing any content already displayed

#### Scenario: Empty state is shown

- **WHEN** a list view contains no items and is not loading
- **THEN** its declared empty state is shown

### Requirement: No-view commands report their outcome

A command declared as `no-view` SHALL NOT emit a view tree. It SHALL report
success or failure, and the launcher SHALL close on success while surfacing the
message on failure.

#### Scenario: No-view command succeeds

- **WHEN** a `no-view` command completes successfully
- **THEN** the launcher hides
- **AND** a brief confirmation is shown

#### Scenario: No-view command fails

- **WHEN** a `no-view` command fails
- **THEN** the launcher stays open and the failure is shown

### Requirement: View stack

Views SHALL form a stack. Invoking an action that opens a view SHALL push onto
the stack, Escape SHALL pop one level, and Escape at the root SHALL hide the
launcher. Hiding the launcher SHALL clear the stack back to root.

#### Scenario: Escape pops one level

- **WHEN** the user is two views deep and presses Escape
- **THEN** the previous view is shown with its prior state intact

#### Scenario: Escape at root hides the launcher

- **WHEN** the user is at the root view and presses Escape
- **THEN** the launcher hides

#### Scenario: Reopening starts at root

- **WHEN** the user is three views deep, hides the launcher, and reopens it
- **THEN** the root view is shown with an empty query
