# background-tasks Specification

## Purpose
Let long jobs, such as installing an update, run independent of the view that
started them. The user can watch a job, leave it, see how it goes from
anywhere in the launcher, cancel it, and learn its result.

## Requirements

### Requirement: A task outlives its view

A task SHALL keep running when the user leaves its progress view, goes back,
or hides the launcher. Only cancelling it, finishing, or quitting Dango SHALL
end it. Running the action or command that started a task while that task runs
SHALL show its progress view again, not start a second task.

#### Scenario: Leaving the progress view

- **WHEN** the user presses Escape on a running task's progress view
- **THEN** the previous view is shown and the task keeps running

#### Scenario: Coming back to a running task

- **WHEN** a task runs and the user runs the action that started it
- **THEN** the task's progress view is shown with its current progress
- **AND** no second task starts

### Requirement: A progress view shows one task

A task's progress view SHALL show the task's title, a subtitle, a progress bar,
a one-line detail, and the task's steps in order, each with a status dot and
its title. The bar SHALL be determinate only when the task reports a share
done. The view SHALL update as the task reports, without losing the user's
place. While the task runs, Enter SHALL leave the view and keep the task
running.

#### Scenario: A running task

- **WHEN** a task is on its second of three steps and reports 62 percent
- **THEN** its progress view shows the bar at 62 percent, the first step done,
  the second working, and the third queued
- **AND** the ActionBar shows "Continue in background" with the Enter key

#### Scenario: A task without a known total

- **WHEN** a task reports progress without a total
- **THEN** its bar is indeterminate

### Requirement: Running tasks show in the ActionBar elsewhere

While a task runs and its own view is not shown, the ActionBar SHALL show
a busy ticker with the task's status line. A task that waits for the user's
input is not running and SHALL show no busy ticker. A ticker that an action raises SHALL
take its place while it shows. The busy-mark timing (see design-system) SHALL
apply.

#### Scenario: Back at root during a download

- **WHEN** an update downloads and the user is at root search
- **THEN** the ActionBar shows a busy ticker reading "Downloading Dango 0.5.0 ·
  42%"

#### Scenario: Two running tasks

- **WHEN** two tasks run and neither task's own view is shown
- **THEN** the busy ticker shows the task that started first

### Requirement: Every running task can be cancelled

Every running task SHALL offer "Cancel" with the chord Ctrl+. on Windows and
Cmd+. on macOS, in its own view and in its busy ticker. Cancelling SHALL end
the task at once for the user, discard what it produced, and leave Dango as it
was before the task started.

#### Scenario: Cancel on Windows

- **WHEN** an update downloads on Windows and the user presses Ctrl+.
- **THEN** the launcher stops showing the download, nothing is installed, and
  the update item is still offered

#### Scenario: Cancel on macOS

- **WHEN** an update downloads on macOS and the user presses Cmd+.
- **THEN** the launcher stops showing the download, nothing is installed, and
  the update item is still offered

### Requirement: A task's result while hidden shows on the next open

A task that finishes, fails, or comes to need input while the launcher is
hidden SHALL show that in the notice at once, as a success or a failure, and as a
ticker the next time the launcher opens: a success ticker when it finished or
needs input, and a failure ticker with its failure sentence when it failed.
When the next open shows the task's own view, the view reports it and no ticker
is shown. When the result offers "Show answer" and the user opens the launcher
while its notice still shows, the answer's view SHALL open and no ticker is shown.

#### Scenario: A download fails while hidden

- **WHEN** an update download fails while the launcher is hidden and the user
  then opens the launcher
- **THEN** the notice showed the failure when it happened, and a failure ticker
  reads "Couldn't download the update. Check your connection and try again."

#### Scenario: An update is ready while hidden

- **WHEN** an update finishes downloading while the launcher is hidden and the
  user then opens the launcher
- **THEN** a success notice read "Dango 0.5.0 is ready to restart" when it
  happened, and a success ticker reads the same

#### Scenario: Reopening onto the task's own view

- **WHEN** an update fails while the launcher is held on its progress view, and
  the user opens the launcher within the grace period
- **THEN** the progress view shows the failure and no failure ticker is shown

#### Scenario: Opening while an answer's notice shows

- **WHEN** an AI answer finishes while the launcher is hidden, and the user
  opens the launcher while its notice still shows
- **THEN** the launcher opens on the answer and no ticker is shown

#### Scenario: Opening after an answer's notice is gone

- **WHEN** an AI answer finishes while the launcher is hidden, and the user
  opens the launcher after its notice is gone
- **THEN** a success ticker reads "Answer ready: " and the command's title, with
  "Show answer"

### Requirement: A task's result while shown appears where the user is

While the launcher is shown, a task's result SHALL appear in its own view
when that view is on top, and as a ticker otherwise: a success ticker when the
task finished or needs input, and a failure ticker with its failure sentence
when it failed.

#### Scenario: Ready while browsing root search

- **WHEN** an update finishes downloading while the user is at root search
- **THEN** a success ticker reads "Dango 0.5.0 is ready to restart"
