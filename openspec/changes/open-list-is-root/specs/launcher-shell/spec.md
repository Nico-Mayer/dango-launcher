## MODIFIED Requirements

### Requirement: Dismissal

The launcher SHALL hide when it loses focus to another application and when the
global shortcut is pressed while it is visible. Both of these are interrupting
hides and SHALL hold the launcher's state as the reset requirement describes.
A click on a transparent part of the launcher window, such as the gap between
the pill and the card, SHALL count as a click outside the launcher.

At the root prompt, Escape SHALL act in stages. With text in the prompt,
Escape SHALL clear the prompt and leave the launcher open. With an empty
prompt and the empty-query list open (see root-search), Escape SHALL close
that list and leave the launcher open, unless `launcher.open-list` is `true`:
then the open list is the root view, and Escape SHALL hide the launcher. With
an empty prompt and the list closed, Escape SHALL hide the launcher. Each hide
by Escape is a deliberate hide. In a pushed view, Escape SHALL pop that view,
whether or not the prompt holds text, as the view stack in view-protocol
describes.

#### Scenario: Escape clears a non-empty prompt

- **WHEN** the launcher shows the root prompt with text in it and the user presses Escape
- **THEN** the prompt is cleared
- **AND** the launcher stays visible, showing the root view as
  `launcher.open-list` sets it

#### Scenario: Escape closes the empty-query list

- **WHEN** `launcher.open-list` is absent, the user has opened the empty-query
  list with Arrow Down, and presses Escape
- **THEN** the list closes and the launcher shows the closed root view
- **AND** the launcher stays visible

#### Scenario: Escape on the open list hides when it is the root view

- **WHEN** `launcher.open-list` is `true`, the prompt is empty, and the user
  presses Escape
- **THEN** the launcher hides
- **AND** the next activation shows the open list again

#### Scenario: Escape on an empty prompt dismisses

- **WHEN** the launcher shows the root prompt, empty, with the list closed,
  and the user presses Escape
- **THEN** the launcher hides
- **AND** focus returns to the application that was in the foreground beforehand

#### Scenario: Escape with only pinned items shown dismisses

- **WHEN** the launcher shows the closed root view with the "Needs attention"
  card and the user presses Escape
- **THEN** the launcher hides

#### Scenario: Escape in a pushed view pops it

- **WHEN** a pushed view is on screen with text in the prompt and the user
  presses Escape
- **THEN** the view is popped and the launcher stays visible

#### Scenario: Clicking away dismisses

- **WHEN** the launcher is visible and the user clicks another application
- **THEN** the launcher hides
- **AND** its state is held for the grace period

#### Scenario: Clicking the gap dismisses

- **WHEN** the launcher shows the pill and the card and the user clicks in the
  gap between them
- **THEN** the launcher hides
- **AND** its state is held for the grace period

#### Scenario: Shortcut toggles

- **WHEN** the launcher is visible and the user presses the global shortcut
- **THEN** the launcher hides
- **AND** its state is held for the grace period

### Requirement: The open list at start is a config setting

Dango SHALL read a boolean from `launcher.open-list` in the config file. When
`true`, every reset and the first show after Dango starts SHALL present the
root view with the empty-query list open (see root-search), and the open list
SHALL stay the root view for as long as the prompt is empty. When absent, it
SHALL mean `false`. A held show SHALL NOT be affected. A changed value SHALL
apply to the next reset without a restart, and the setting SHALL appear in the
generated schema and in the settings reference.

#### Scenario: Setting is absent

- **WHEN** the config file does not mention `launcher.open-list` and the
  launcher opens after a reset
- **THEN** the root view is closed

#### Scenario: Setting is on

- **WHEN** `launcher.open-list` is `true` and the launcher opens after a reset
- **THEN** the empty-query list is open

#### Scenario: A held show keeps its state

- **WHEN** `launcher.open-list` is `true`, the user types `fi`, and focus
  moves to another application
- **AND** the user activates the launcher within the grace period
- **THEN** the prompt still reads `fi` and its results are shown

#### Scenario: Setting is changed while running

- **WHEN** the user sets `launcher.open-list` to `true` and saves the file
  while Dango is running
- **THEN** the launcher opens with the list open after the next reset, with no
  restart

#### Scenario: Escape hides from the open list

- **WHEN** `launcher.open-list` is `true` and the launcher opens after a reset
- **AND** the user presses Escape
- **THEN** the launcher hides, with no stage that closes the list first

#### Scenario: The setting is documented

- **WHEN** the settings reference is generated
- **THEN** it lists `launcher.open-list` with its type, default, and
  description
