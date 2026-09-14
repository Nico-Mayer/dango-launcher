## MODIFIED Requirements

### Requirement: A hotkey invokes a command the same way root search does

Pressing a command's hotkey SHALL have the same effect as selecting that command
in root search and confirming it. A command that shows no view SHALL run without
the launcher appearing; a command that shows a view SHALL show the launcher with
that view.

Sameness SHALL extend to what the user can then do: a view opened by a hotkey
SHALL accept confirmation, per-action shortcuts, and its action panel exactly as
the same view does when it is opened from root search.

#### Scenario: A no-view command runs without the launcher

- **WHEN** the user presses the hotkey for a command that shows no view
- **THEN** the command runs and the launcher does not appear

#### Scenario: A view command shows its view

- **WHEN** the user presses the hotkey for a command that shows a view
- **THEN** the launcher appears showing that view

#### Scenario: Confirming in a hotkey-opened view

- **WHEN** the user presses the hotkey for a view command and confirms a row in
  the view that appears
- **THEN** that row's primary action runs

#### Scenario: The action panel in a hotkey-opened view

- **WHEN** the user opens the action panel in a hotkey-opened view and chooses
  an action
- **THEN** that action runs
