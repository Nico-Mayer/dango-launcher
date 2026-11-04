## MODIFIED Requirements

### Requirement: The notice reports only outcomes the user cannot see

While the launcher is hidden, Dango SHALL show the notice for a failure, for a
copy that closed the launcher, for a no-view command that succeeded with a
message, and for a background task that finishes or comes to need input. It
SHALL NOT show the notice for an outcome that is visible on its own, such as a
moved window, a launched application, or a locked screen. While the launcher is
visible, its ActionBar ticker SHALL report instead and the notice SHALL NOT
show.

#### Scenario: A hotkey command fails

- **WHEN** the user presses the hotkey for a no-view command and the command
  fails
- **THEN** the notice shows the failure's message
- **AND** the launcher does not appear

#### Scenario: An action fails after hiding the launcher

- **WHEN** an action hides the launcher to paste and pasting fails
- **THEN** the notice shows the failure's message

#### Scenario: A held command fails while hidden

- **WHEN** a command keeps running after an interrupting hide and fails before
  the launcher is shown again
- **THEN** the notice shows the failure's message

#### Scenario: A copy closes the launcher

- **WHEN** the user runs an action that copies to the clipboard and hides the
  launcher
- **THEN** the notice reads "Copied to clipboard"

#### Scenario: A no-view command reports a level

- **WHEN** the user presses the hotkey for `Increase Brightness` and it succeeds
  with "Brightness set to 70%"
- **THEN** a success notice reads "Brightness set to 70%"
- **AND** the launcher does not appear

#### Scenario: A background task finishes while hidden

- **WHEN** the AI command "Summarize" finishes in the background while the
  launcher is hidden
- **THEN** a success notice reads "Answer ready: Summarize"

#### Scenario: A visible outcome shows nothing

- **WHEN** the user presses the hotkey for a window-management command and the
  window moves
- **THEN** no notice shows

#### Scenario: The launcher is visible

- **WHEN** an action fails while the launcher is on screen
- **THEN** the failure shows as a ticker in the ActionBar
- **AND** no notice shows
