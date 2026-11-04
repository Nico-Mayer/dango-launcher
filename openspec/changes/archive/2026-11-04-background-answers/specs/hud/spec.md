# Spec Delta

## MODIFIED Requirements

### Requirement: The HUD reports only outcomes the user cannot see

While the launcher is hidden, Dango SHALL show the HUD for a failure, for a
copy that closed the launcher, and for a background task that finishes or
comes to need input. It SHALL NOT show the HUD for an outcome that is visible
on its own, such as a moved window, a launched application, or a locked
screen. While the launcher is visible, its ActionBar toast SHALL report instead
and the HUD SHALL NOT show.

#### Scenario: A hotkey command fails

- **WHEN** the user presses the hotkey for a no-view command and the command
  fails
- **THEN** the HUD shows the failure's message
- **AND** the launcher does not appear

#### Scenario: An action fails after hiding the launcher

- **WHEN** an action hides the launcher to paste and pasting fails
- **THEN** the HUD shows the failure's message

#### Scenario: A held command fails while hidden

- **WHEN** a command keeps running after an interrupting hide and fails before
  the launcher is shown again
- **THEN** the HUD shows the failure's message

#### Scenario: A copy closes the launcher

- **WHEN** the user runs an action that copies to the clipboard and hides the
  launcher
- **THEN** the HUD reads "Copied to clipboard"

#### Scenario: A background task finishes while hidden

- **WHEN** the AI command "Summarize" finishes in the background while the
  launcher is hidden
- **THEN** a success HUD reads "Answer ready: Summarize"

#### Scenario: A visible outcome shows nothing

- **WHEN** the user presses the hotkey for a window-management command and the
  window moves
- **THEN** no HUD shows

#### Scenario: The launcher is visible

- **WHEN** an action fails while the launcher is on screen
- **THEN** the failure shows as a toast in the ActionBar
- **AND** no HUD shows

### Requirement: The HUD disappears on its own

A success HUD SHALL disappear after about two seconds and a failure HUD after
about four. A new HUD SHALL replace the one showing and start its own time.
Showing the launcher SHALL remove the HUD at once. When the user opens the
launcher, with its hotkey or the tray menu, while the HUD reports a result that
offers "Show answer", the launcher SHALL open on that answer. The HUD SHALL NOT
wait for a key press or a click.

#### Scenario: A success HUD times out

- **WHEN** a success HUD shows
- **THEN** it is gone after about two seconds

#### Scenario: A failure HUD times out

- **WHEN** a failure HUD shows
- **THEN** it stays about four seconds and then is gone

#### Scenario: A newer HUD replaces the old one

- **WHEN** a HUD is showing and another outcome is reported
- **THEN** only the newer HUD shows, for its own full time

#### Scenario: The launcher opens over a HUD

- **WHEN** a HUD that offers no answer is showing and the user opens the
  launcher
- **THEN** the HUD is gone and the launcher shows without the HUD's message

#### Scenario: The launcher opens over an answer's HUD

- **WHEN** the HUD reads "Answer ready: Summarize" and the user opens the
  launcher before it is gone
- **THEN** the HUD is gone and the answer's view opens, with actions to paste
  or copy it and no toast

#### Scenario: A command hotkey during an answer's HUD

- **WHEN** the HUD reads "Answer ready: Summarize" and the user presses the
  hotkey of a command that shows the launcher
- **THEN** that command runs and the answer does not open
