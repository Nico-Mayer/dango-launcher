## MODIFIED Requirements

### Requirement: The selected text can be read

Dango SHALL be able to read the text currently selected in the frontmost
application, and SHALL report the absence of a selection distinctly from a
failure to read one.

On both platforms Dango SHALL first ask the application directly, and SHALL
send no keystrokes to do it. The clipboard round trip SHALL be a fallback, used
only where the application cannot be asked.

An application that answers but reports nothing selected SHALL be treated as an
empty selection, not as a failure to read, and SHALL NOT fall through to the
keystroke path. An answer from Dango's own window, or an empty answer from an
element that exposes no selection at all, SHALL count as no answer. The direct
request SHALL be bounded in time, so an application that does not answer
cannot hold up the read.

#### Scenario: Reading a selection on macOS

- **WHEN** text is selected in the frontmost application and Dango reads the selection
- **THEN** that text is returned
- **AND** the clipboard's contents are unchanged

#### Scenario: Reading a selection on Windows

- **WHEN** text is selected in the frontmost application, that application
  exposes it, and Dango reads the selection
- **THEN** that text is returned
- **AND** no keystroke is sent to the application
- **AND** the clipboard's contents are unchanged

#### Scenario: Nothing is selected

- **WHEN** Dango reads the selection and the frontmost application has none
- **THEN** an empty selection is reported
- **AND** anything acting on the selection says so rather than acting on empty text

#### Scenario: The application answers and has no selection

- **WHEN** the frontmost application exposes its text but nothing is selected,
  on either platform
- **THEN** an empty selection is reported
- **AND** no keystroke is sent

#### Scenario: The application will not say what is selected

- **WHEN** the frontmost application cannot be asked what is selected
- **THEN** Dango falls back to reading it through the clipboard
- **AND** the clipboard holds what it held before the read, once the read has
  finished

#### Scenario: Dango's own search field is never the answer on macOS

- **WHEN** the launcher is on screen on macOS, the application behind it
  exposes nothing, and Dango reads the selection
- **THEN** the empty search field of the launcher is not reported as an empty
  selection
- **AND** Dango falls back to reading it through the clipboard

#### Scenario: An element that exposes no selection on macOS

- **WHEN** the focused element of the frontmost application on macOS answers
  with empty selected text but exposes no selected text range
- **THEN** Dango falls back to reading it through the clipboard

#### Scenario: Reading a selection is fast enough to act on

- **WHEN** a selection of 1,000 characters is read
- **THEN** it is returned without visible delay

#### Scenario: An application that does not answer in time

- **WHEN** the frontmost application has not answered the direct request after
  a short bounded wait, on either platform
- **THEN** Dango stops waiting for it
- **AND** the read completes by the fallback or reports that it could not be done

## ADDED Requirements

### Requirement: Reading the selection keeps the launcher on screen

Reading the selection SHALL NOT hide the launcher, on either platform, whether
the command was run from root search or from its own hotkey. When the read is
done, keyboard input SHALL go to the launcher again, to the field that had it
before. A failed read SHALL be shown in the launcher at once.

#### Scenario: Reading on Windows

- **WHEN** the launcher is on screen on Windows and the user runs a command
  that reads the selection
- **THEN** the launcher stays on screen during the read
- **AND** once the read is done, typing goes to the launcher

#### Scenario: Reading on macOS

- **WHEN** the launcher is on screen on macOS and the user runs a command that
  reads the selection
- **THEN** the launcher stays on screen during the read
- **AND** typing still goes to the launcher

#### Scenario: The copy fallback on Windows

- **WHEN** the application behind the launcher on Windows exposes nothing and
  the selection is read through the clipboard
- **THEN** the launcher stays on screen
- **AND** the copy reaches the application behind it, not the launcher

#### Scenario: The copy fallback on macOS

- **WHEN** the application behind the launcher on macOS exposes nothing and
  the selection is read through the clipboard
- **THEN** the launcher stays on screen
- **AND** the copy reaches the application behind it, not the launcher

#### Scenario: A failed read is shown at once

- **WHEN** a command reads the selection and nothing is selected
- **THEN** the launcher is still on screen and shows "Select some text first"

#### Scenario: Lending the foreground is not a focus loss on Windows

- **WHEN** Dango gives the foreground to the previous window to read the
  selection
- **THEN** the launcher is not hidden as if the user had clicked away

### Requirement: The copy fallback uses a chord that copies everywhere

The keystroke Dango sends to read a selection through the clipboard SHALL be
one that means copy in editors and terminals alike, and SHALL NOT interrupt a
running program or run an editor command. On Windows it SHALL be Ctrl+Insert.
On macOS it SHALL be Cmd+C.

#### Scenario: The copy chord on Windows

- **WHEN** Dango reads a selection through the clipboard on Windows
- **THEN** it sends Ctrl+Insert and never Ctrl+C

#### Scenario: An editor that copies on Ctrl+Insert

- **WHEN** text is selected in Zed on Windows and an AI command that needs the
  selection is run
- **THEN** the selected text is sent with the command's prompt

#### Scenario: The copy chord on macOS

- **WHEN** Dango reads a selection through the clipboard on macOS
- **THEN** it sends Cmd+C to the application behind the launcher

## REMOVED Requirements

### Requirement: The keystroke fallback can be refused for an application

**Reason**: The list existed because Ctrl+C is destructive in some editors. The
Windows fallback now sends Ctrl+Insert, which has no such meaning, and a
per-application list does not scale.

**Migration**: Delete `selection.excluded-applications` from the config file.
The key is no longer read. Alpha stage, so no compatibility shim.
