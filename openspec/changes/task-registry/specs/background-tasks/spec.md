# Spec Delta

## Purpose

Let long jobs, such as installing an update, run independent of the view that
started them. The user can watch a job, leave it, see how it goes from
anywhere in the launcher, cancel it, and learn its result.

## ADDED Requirements

### Requirement: A task outlives its view

A task SHALL keep running when the user leaves its progress view, goes back,
or hides the launcher. Only cancelling it, finishing, or quitting Dango SHALL
end it. Running the command that started a task while that task runs SHALL show
its progress view again, not start a second task.

#### Scenario: Leaving the progress view

- **WHEN** the user presses Escape on a running task's progress view
- **THEN** the previous view is shown and the task keeps running

#### Scenario: Coming back to a running task

- **WHEN** a task runs and the user runs the command that started it
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

- **WHEN** a task is on its second of four steps and reports 62 percent
- **THEN** its progress view shows the bar at 62 percent, the first step done,
  the second working, and the last two queued
- **AND** the ActionBar shows "Continue in background" with the Enter key

#### Scenario: A task without a known total

- **WHEN** a task reports progress without a total
- **THEN** its bar is indeterminate

### Requirement: Running tasks show in the ActionBar elsewhere

While a task runs and its progress view is not shown, the ActionBar SHALL show
a busy toast with the task's status line. A toast that an action raises SHALL
take its place while it shows. The busy-mark timing (see design-system) SHALL
apply.

#### Scenario: Back at root during a download

- **WHEN** an update downloads and the user is at root search
- **THEN** the ActionBar shows a busy toast reading "Downloading Dango 0.5.0 ·
  42%"

#### Scenario: Two running tasks

- **WHEN** two tasks run and neither progress view is shown
- **THEN** the busy toast shows the task that started first

### Requirement: Every running task can be cancelled

Every running task SHALL offer "Cancel" with the chord Ctrl+. on Windows and
Cmd+. on macOS, in its progress view and in its busy toast. Cancelling SHALL
stop the task's work as soon as it can, discard what it produced, and leave
Dango as it was before the task started.

#### Scenario: Cancel on Windows

- **WHEN** an update downloads on Windows and the user presses Ctrl+.
- **THEN** the download stops, nothing is installed, and the update item is
  still offered

#### Scenario: Cancel on macOS

- **WHEN** an update downloads on macOS and the user presses Cmd+.
- **THEN** the download stops, nothing is installed, and the update item is
  still offered

### Requirement: A task that ends while hidden reports on the next open

A task that finishes, fails, or needs input while the launcher is hidden SHALL
show its result as a toast the next time the launcher opens: a success toast
when it finished or needs input, and a failure toast with its failure sentence
when it failed. A task that ends while the launcher is shown SHALL report in
its progress view when that is shown, and as a toast otherwise.

#### Scenario: A download fails while hidden

- **WHEN** an update download fails while the launcher is hidden and the user
  then opens the launcher
- **THEN** a failure toast reads "Couldn't download the update. Check your
  connection and try again."

#### Scenario: An update is ready while hidden

- **WHEN** an update finishes downloading while the launcher is hidden and the
  user then opens the launcher
- **THEN** a success toast reads "Dango 0.5.0 is ready to install"
