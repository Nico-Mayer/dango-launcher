## Purpose

The built-in AI extension: a set of one-shot text transforms the user reaches by
name or by hotkey, each one a prompt the user can read and change, with the
answer streaming back into the launcher and going wherever the command says.

## ADDED Requirements

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
the answer is complete.

#### Scenario: A view appears before the answer does

- **WHEN** an AI command is invoked
- **THEN** a view is on screen within 100ms showing that it is working
- **AND** it is replaced by the answer as the answer arrives

#### Scenario: Text arrives progressively

- **WHEN** the model is producing a long answer
- **THEN** the text on screen grows as it arrives
- **AND** the view is updated at most once every 50ms rather than on every
  fragment

#### Scenario: The launcher stays responsive while streaming

- **WHEN** an answer is streaming
- **THEN** the action panel opens and key presses are handled without delay

#### Scenario: The answer is complete

- **WHEN** the model has finished
- **THEN** the in-progress indication is gone
- **AND** the whole answer is on screen

### Requirement: A running request can be abandoned

The user SHALL be able to stop a request that is running. Escape SHALL leave the
view the way it leaves any view, and an abandoned request's later output SHALL
NOT appear.

#### Scenario: Escape abandons the request

- **WHEN** an answer is streaming and the user presses Escape
- **THEN** the view is closed
- **AND** no further text from that request is shown

#### Scenario: Hiding the launcher abandons the request

- **WHEN** an answer is streaming and the launcher is hidden
- **THEN** the request is abandoned and nothing from it is shown later

#### Scenario: Running another command supersedes it

- **WHEN** the user runs a second command while an answer is still streaming
- **THEN** only the second command's output is shown

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
