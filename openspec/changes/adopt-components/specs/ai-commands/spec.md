# Spec Delta

## ADDED Requirements

### Requirement: A running request is a background task

Once its prompt is ready, a request SHALL run as a background task (see
background-tasks). Escape and hiding the launcher SHALL NOT stop it. Its view
SHALL offer "Continue in background" with Enter and "Cancel" with the cancel
chord. Running the same command again while it runs SHALL show its view again;
running another AI command SHALL cancel it once that command's prompt is ready.

#### Scenario: Escape leaves the request running

- **WHEN** an answer is streaming and the user presses Escape
- **THEN** the view is closed and the busy toast reads "Running" followed by the
  command's title

#### Scenario: Cancel on Windows

- **WHEN** a request runs and the user presses Ctrl+. in its view or on its busy
  toast
- **THEN** the request stops and nothing from it is shown later

#### Scenario: Cancel on macOS

- **WHEN** a request runs and the user presses Cmd+. in its view or on its busy
  toast
- **THEN** the request stops and nothing from it is shown later

#### Scenario: Running the same command again

- **WHEN** a request runs in the background and the user runs that command
  again
- **THEN** its view shows the answer so far, and no second request starts

#### Scenario: Running another command supersedes it

- **WHEN** the user runs a second AI command while an answer is still streaming
- **THEN** the first request is cancelled and only the second command's output
  is shown

#### Scenario: A form does not stop a running request

- **WHEN** a request runs in the background and the user opens another AI
  command's argument form and leaves it
- **THEN** the first request keeps running

### Requirement: A request that finishes away is kept, not pasted

A request finishes away when its view was left or the launcher is hidden. Then
an answer to show or paste SHALL NOT be pasted: it SHALL be kept, and a success
toast SHALL read "Answer ready: " and the title, offering "Show answer" with
Ctrl+Enter on Windows and Cmd+Enter on macOS. An answer to copy SHALL be copied,
with the toast "Copied to clipboard". A failure SHALL be reported as
background-tasks says.

#### Scenario: A paste command finishes after Escape

- **WHEN** "Summarize" is set to paste and the user pressed Escape before its
  answer finished
- **THEN** nothing is pasted, and a toast reads "Answer ready: Summarize" with
  "Show answer"

#### Scenario: A paste command finishes while the launcher is held

- **WHEN** a paste command runs, the user clicks into another application, and
  the answer finishes within the grace period
- **THEN** nothing is pasted and the toast shows when the launcher next opens

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

## REMOVED Requirements

### Requirement: A running request can be abandoned

**Reason**: A request now outlives its view as a background task, so Escape and
a hide no longer abandon it.
**Migration**: See "A running request is a background task". Cancel stops a
request; Escape only leaves its view.
