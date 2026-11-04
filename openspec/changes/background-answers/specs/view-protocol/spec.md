# Spec Delta

## MODIFIED Requirements

### Requirement: Action panel

Any view SHALL be able to declare a list of actions, the first of which is the
primary action. The primary action SHALL be invocable with Enter and the panel
SHALL be openable to reach the remaining actions. Each action MAY declare its
own keyboard shortcut, a section, and a style of normal or danger.

The panel SHALL list the actions in order, grouping those that declare a
section under that section's heading, and SHALL show danger actions in the
danger role after all others. The panel SHALL have its own filter input: typing
while it is open SHALL narrow the actions by title, select the first match, and
SHALL NOT change the query below. When nothing matches, the panel SHALL say so.

The panel SHALL open on Ctrl+K on Windows and on Cmd+K or Ctrl+K on macOS, for
the selected row, and that chord SHALL do nothing else. Ctrl combined with J, K,
N, or P SHALL NOT move the selection in any list; only the arrow keys move it.

Wherever the launcher shows a chord, it SHALL name the modifier the platform
uses: Cmd on macOS and Ctrl on Windows. The ActionBar SHALL name the action
Enter runs: a list's or grid's selected item's primary action, or the view's
own primary action.

#### Scenario: Primary action runs on Enter

- **WHEN** a view declares actions and the user presses Enter
- **THEN** the first action is invoked

#### Scenario: Secondary action is reachable

- **WHEN** the user opens the action panel and chooses a secondary action
- **THEN** that action is invoked

#### Scenario: View declares no actions

- **WHEN** a view declares no actions and the user presses Enter
- **THEN** nothing is invoked and the view is unchanged

#### Scenario: Opening the panel on Windows

- **WHEN** a row with more than one action is selected and the user presses
  Ctrl+K
- **THEN** the action panel opens listing that row's actions
- **AND** the selected row is the same row as before the press

#### Scenario: Opening the panel on macOS

- **WHEN** a row with more than one action is selected and the user presses
  Cmd+K or Ctrl+K
- **THEN** the action panel opens listing that row's actions
- **AND** the selected row is the same row as before the press

#### Scenario: Ctrl with a letter does not move the selection

- **WHEN** a list is showing and the user presses Ctrl+J, Ctrl+N, or Ctrl+P
- **THEN** the selected row is unchanged
- **AND** the action panel does not open

#### Scenario: Sections and danger actions

- **WHEN** an item declares "Pin entry" and "Delete entry" in a section "Manage",
  with "Delete entry" in the danger style
- **THEN** the panel shows them under the heading "Manage", with "Delete entry"
  last and in the danger role

#### Scenario: Filtering the panel

- **WHEN** the panel is open and the user types "co"
- **THEN** only actions whose titles match "co" are listed, the first is
  selected, and the query below is unchanged

#### Scenario: Nothing matches in the panel

- **WHEN** the panel's filter matches no action
- **THEN** the panel shows "No matching actions" and Enter runs nothing

#### Scenario: The footer names the platform's chord on macOS

- **WHEN** the launcher is open on macOS
- **THEN** the ActionBar shows the action panel chord with the Cmd symbol
- **AND** an action's shortcut in the panel is shown with the Cmd symbol

#### Scenario: The footer names the platform's chord on Windows

- **WHEN** the launcher is open on Windows
- **THEN** the ActionBar shows the action panel chord as "Ctrl K"
- **AND** an action's shortcut in the panel is shown with Ctrl

#### Scenario: The footer names the primary action of a list

- **WHEN** a list view is showing with a selected row that has actions
- **THEN** the ActionBar names that row's first action next to the Enter key

#### Scenario: The rule holds in root search and in a pushed list

- **WHEN** the user presses Ctrl+K in root search, or in a list view a command
  pushed
- **THEN** in both places the panel opens for the selected row and the selection
  does not move
