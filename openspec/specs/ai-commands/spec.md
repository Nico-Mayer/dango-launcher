# ai-commands Specification

## Purpose

The built-in AI extension: a set of one-shot text transforms the user reaches by
name or by hotkey, each one a prompt the user can read and change, with the
answer streaming back into the launcher and going wherever the command says.

## Requirements

### Requirement: A set of text transforms ships built in

The extension SHALL contribute a set of transform commands that work with no
configuration beyond a provider: Improve Writing, Fix Spelling and Grammar, Make
Shorter, Make Longer, Make Simpler, Make Professional, Summarize, Explain This,
and Translate. Each SHALL be found in root search by its title and SHALL act on
the text the user has selected in the application they came from.

#### Scenario: A shipped command is found by name

- **WHEN** the user types part of a shipped command's title in root search
- **THEN** that command appears in the results

#### Scenario: A shipped command transforms the selection

- **WHEN** text is selected in another application and the user runs Improve
  Writing
- **THEN** the selected text is sent with that command's prompt
- **AND** the answer appears in the launcher

#### Scenario: Nothing is selected

- **WHEN** the user runs a command whose prompt needs the selection and nothing
  is selected
- **THEN** no request is made
- **AND** the launcher says to select some text first

#### Scenario: A command that asks for something

- **WHEN** the user runs Translate, whose prompt asks which language to use
- **THEN** a form asking for it is shown before any request is made
- **AND** submitting it runs the command with that value

#### Scenario: Abandoning the form sends nothing

- **WHEN** the user dismisses that form without submitting
- **THEN** no request is made

### Requirement: A command is a prompt the user can change

Every AI command SHALL be defined by a prompt written as a template, using the
same placeholders as the rest of Dango: `selection`, `clipboard`, and any other
name, which becomes something the user is asked for. The shipped commands SHALL
be defaults of exactly this shape, so changing one's prompt in the configuration
file changes what it sends.

#### Scenario: A prompt uses the selection

- **WHEN** a command's prompt refers to the selection
- **THEN** the selected text is put in its place before the request is made

#### Scenario: A prompt uses the clipboard

- **WHEN** a command's prompt refers to the clipboard
- **THEN** the clipboard's text is put in its place

#### Scenario: The user rewrites a shipped command's prompt

- **WHEN** the user sets a different prompt for a shipped command in the
  configuration file
- **THEN** running that command sends the user's prompt instead

#### Scenario: A prompt that cannot be read

- **WHEN** a command's prompt cannot be parsed as a template
- **THEN** that command is not offered
- **AND** the problem is surfaced the way a configuration problem is

### Requirement: A command can be added, renamed, and removed in configuration

The user SHALL be able to define a new AI command in the configuration file with
its own title and prompt, give it an alias and a global hotkey through the
existing per-command settings, and turn off a shipped command they do not want.
A command added, changed, or removed in the file SHALL take effect without a
restart.

#### Scenario: A command added by hand appears

- **WHEN** the user adds an AI command with a title and a prompt and saves the
  file
- **THEN** it appears in root search under that title, without a restart

#### Scenario: A user-defined command gets a hotkey

- **WHEN** the user gives an AI command a global hotkey
- **THEN** pressing that chord runs the command from anywhere, the way any other
  command's hotkey does

#### Scenario: A shipped command is turned off

- **WHEN** the user marks a shipped command as disabled
- **THEN** it no longer appears in root search
- **AND** the other AI commands are unaffected

#### Scenario: A command removed from the file goes away

- **WHEN** the user deletes a command they added and saves the file
- **THEN** it disappears from root search and its hotkey is released, without a
  restart

#### Scenario: A command with no prompt is refused

- **WHEN** the user defines a command with a title but no prompt
- **THEN** that command is not offered and the problem is surfaced
- **AND** the rest of the AI commands still work

### Requirement: The answer streams into the launcher

Running an AI command SHALL put a view on screen that fills in as the answer
arrives, rather than waiting for the whole answer. The launcher SHALL stay
responsive throughout, and the view SHALL show that work is in progress until
the answer is complete. Until the first text arrives, the view SHALL name the
provider it is waiting for, by the name the user gave it in the config file.

#### Scenario: A view appears before the answer does

- **WHEN** an AI command using the provider named `work` is invoked
- **THEN** a view reading "Waiting for work…" is on screen at once, without
  waiting for the provider
- **AND** it is replaced by the answer as the answer arrives

#### Scenario: Text arrives progressively

- **WHEN** the model is producing a long answer
- **THEN** the text on screen grows as it arrives

#### Scenario: The launcher stays responsive while streaming

- **WHEN** an answer is streaming
- **THEN** the action panel opens and key presses are handled without delay

#### Scenario: The answer is complete

- **WHEN** the model has finished
- **THEN** the in-progress indication is gone
- **AND** the whole answer is on screen

### Requirement: What happens to the answer is the command's choice

Each command SHALL declare what becomes of its answer: shown in the launcher,
pasted into the application the user came from, or copied to the clipboard.
Showing it SHALL be the default. Whatever the setting, the result view SHALL
offer pasting and copying as actions.

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

### Requirement: An AI command is a command like any other

The extension's commands SHALL behave as every other command does: they are
enabled and disabled with their extension, they are matched by title, keywords,
and alias, they rank by use, and they can be invoked from root search or from
their own hotkey.

#### Scenario: Disabling the extension removes the commands

- **WHEN** the AI extension is disabled in the configuration file
- **THEN** its commands are gone from root search and their hotkeys are released

#### Scenario: A command invoked from its hotkey behaves the same

- **WHEN** an AI command is run from its global hotkey while another application
  is focused
- **THEN** the launcher shows the streaming view
- **AND** the answer goes where the command says, exactly as when run from root
  search

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

### Requirement: A kept answer opens once

Running a command whose answer was kept SHALL open that answer instead of
starting a new request. A kept answer SHALL be dropped once shown, from the
toast or by running the command again.

#### Scenario: Running the command again after it finished away

- **WHEN** an answer was kept and the user runs that command again
- **THEN** the kept answer's view opens and no new request starts

#### Scenario: Running it once more

- **WHEN** the kept answer was shown and the user runs that command again
- **THEN** a new request starts
