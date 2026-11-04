# Spec Delta

## MODIFIED Requirements

### Requirement: A request that finishes away is kept, not pasted

A request finishes away when its view was left or the launcher is hidden. Then
an answer to show or paste SHALL NOT be pasted: it SHALL be kept, and a success
toast SHALL read "Answer ready: " and the title, offering "Show answer" with
Ctrl+Enter on Windows and Cmd+Enter on macOS. An answer to copy SHALL be
copied, with the toast "Copied to clipboard". A failure SHALL be reported as
background-tasks says. When the request's view is held behind a hide, that
view SHALL show the answer, and reopening onto it SHALL show no toast.

#### Scenario: A paste command finishes after Escape

- **WHEN** "Summarize" is set to paste and the user pressed Escape before its
  answer finished
- **THEN** nothing is pasted, and a toast reads "Answer ready: Summarize" with
  "Show answer"

#### Scenario: A paste command finishes while the launcher is held

- **WHEN** a paste command runs, the user clicks into another application, and
  the answer finishes within the grace period
- **THEN** nothing is pasted, and when the launcher next opens it shows the
  finished answer with no toast

#### Scenario: Showing the answer on Windows

- **WHEN** the "Answer ready" toast shows on Windows and the user presses
  Ctrl+Enter
- **THEN** the answer's view opens with actions to paste or copy it

#### Scenario: Showing the answer on macOS

- **WHEN** the "Answer ready" toast shows on macOS and the user presses
  Cmd+Enter
- **THEN** the answer's view opens with actions to paste or copy it

#### Scenario: Copying away

- **WHEN** a command set to copy finishes after its view was left
- **THEN** the answer is on the clipboard and a toast reads "Copied to
  clipboard"

#### Scenario: Finishing in view still pastes

- **WHEN** a paste command started from its hotkey finishes while its view is
  still on screen
- **THEN** the launcher hides and the answer is inserted, as before
