# Spec Delta

## ADDED Requirements

### Requirement: A running request is a background task

A running request SHALL be a background task (see background-tasks). Escape
SHALL leave its view the way it leaves any view, and leaving or hiding the
launcher SHALL NOT stop the request. Cancel SHALL stop it, and a cancelled
request's later output SHALL NOT appear. Running the same command again while
it runs SHALL show its view again; running a different AI command SHALL cancel
it.

#### Scenario: Escape leaves the request running

- **WHEN** an answer is streaming and the user presses Escape
- **THEN** the view is closed and the busy toast reads "Running" followed by the
  command's title

#### Scenario: Cancel on Windows

- **WHEN** a request runs in the background and the user presses Ctrl+. on its
  busy toast
- **THEN** the request stops and nothing from it is shown later

#### Scenario: Cancel on macOS

- **WHEN** a request runs in the background and the user presses Cmd+. on its
  busy toast
- **THEN** the request stops and nothing from it is shown later

#### Scenario: Running the same command again

- **WHEN** a request runs in the background and the user runs that command
  again
- **THEN** its view shows the answer so far, and no second request starts

#### Scenario: Running another command supersedes it

- **WHEN** the user runs a second AI command while an answer is still streaming
- **THEN** the first request is cancelled and only the second command's output
  is shown

## MODIFIED Requirements

### Requirement: What happens to the answer is the command's choice

Each command SHALL declare what becomes of its answer: shown in the launcher,
pasted into the application the user came from, or copied to the clipboard.
Showing it SHALL be the default. Whatever the setting, the result view SHALL
offer pasting and copying as actions. When a request finishes while its view
is not on top, an answer to show or paste SHALL NOT be pasted. Instead a
success toast SHALL read the command's title followed by "answer is ready" and
offer "Show answer" with Ctrl+Enter on Windows and Cmd+Enter on macOS. An
answer to copy SHALL be copied.

#### Scenario: The answer is shown

- **WHEN** a command whose output is to be shown finishes
- **THEN** the answer stays on screen with actions to paste or copy it

#### Scenario: The answer is pasted on macOS

- **WHEN** a command set to paste finishes and the user came from an application
  with an editable field focused
- **THEN** the launcher hides and the answer is inserted there
- **AND** the text the user had selected is replaced by it

#### Scenario: The answer is pasted on Windows

- **WHEN** a command set to paste finishes and the user came from an application
  with an editable field focused
- **THEN** the launcher hides, the previous window is the foreground window, and
  the answer is inserted there
- **AND** the text the user had selected is replaced by it

#### Scenario: The answer is copied

- **WHEN** a command set to copy finishes
- **THEN** the answer is on the clipboard and the launcher hides

#### Scenario: Pasting from the result view

- **WHEN** the user chooses to paste from a shown answer
- **THEN** the launcher hides and the answer is inserted into the application
  they came from

#### Scenario: The answer cannot be pasted

- **WHEN** pasting fails, because there is no application to return to or the
  permission is missing
- **THEN** the launcher stays open with the answer still on screen
- **AND** the failure says what went wrong and what to do

#### Scenario: An empty answer is not pasted

- **WHEN** the model returns nothing
- **THEN** nothing is inserted or copied
- **AND** the launcher says the model returned no text

#### Scenario: An answer finishes in the background

- **WHEN** "Summarize" is set to paste and its answer finishes while the user is
  at root search
- **THEN** nothing is pasted, and a toast reads "Summarize answer is ready" with
  "Show answer"

#### Scenario: Showing a background answer

- **WHEN** that toast shows and the user presses its chord
- **THEN** the answer's view opens with actions to paste or copy it

#### Scenario: Copying in the background

- **WHEN** a command set to copy finishes while its view is not on top
- **THEN** the answer is on the clipboard and a toast reads "Copied to
  clipboard"

## REMOVED Requirements

### Requirement: A running request can be abandoned

**Reason**: A request now outlives its view as a background task, so Escape and
a hide no longer abandon it.
**Migration**: See "A running request is a background task". Cancel stops a
request; Escape only leaves its view.
