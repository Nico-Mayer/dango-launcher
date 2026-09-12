# system-commands Specification

## Purpose

The built-in extension contributing the everyday operating-system actions a
launcher is expected to have: locking the screen, putting the machine to sleep,
emptying the trash, and quitting a running application.

## Requirements

### Requirement: System commands are searchable and invocable

The `system` extension SHALL contribute lock screen, sleep, empty trash, and
quit application as commands, searchable in root search like any other command.

#### Scenario: Command is found by name

- **WHEN** the user types part of a system command's title
- **THEN** that command appears in the results

#### Scenario: Command is found by keyword

- **WHEN** the user types a keyword a system command declares but that is absent from its title
- **THEN** that command appears in the results

#### Scenario: Extension can be disabled

- **WHEN** the `system` extension is disabled
- **THEN** none of its commands appear in any query

### Requirement: Lock screen

Lock screen SHALL be a no-view command that locks the session immediately,
leaving running applications untouched.

#### Scenario: Lock on macOS

- **WHEN** lock screen is invoked on macOS
- **THEN** the session locks and the login prompt is shown

#### Scenario: Lock on Windows

- **WHEN** lock screen is invoked on Windows
- **THEN** the session locks and the lock screen is shown

#### Scenario: Launcher gets out of the way

- **WHEN** lock screen is invoked
- **THEN** the launcher hides before the screen locks, so it is not on screen when the session is unlocked

### Requirement: Sleep

Sleep SHALL be a no-view command that puts the machine into its normal sleep
state.

#### Scenario: Sleep on macOS

- **WHEN** sleep is invoked on macOS
- **THEN** the machine enters sleep

#### Scenario: Sleep on Windows

- **WHEN** sleep is invoked on Windows
- **THEN** the machine enters sleep

#### Scenario: Sleep is refused

- **WHEN** the operating system refuses the sleep request
- **THEN** the launcher stays open and reports the failure

### Requirement: Empty trash

Empty trash SHALL permanently delete the contents of the platform's trash.
Because the action cannot be undone, it SHALL ask for confirmation before
deleting anything, and the confirmation SHALL name what is about to be lost.

#### Scenario: Confirmation is shown first

- **WHEN** the user invokes empty trash and the trash is not empty
- **THEN** a confirmation is shown stating how many items will be permanently deleted
- **AND** nothing is deleted yet

#### Scenario: Deletion is confirmed

- **WHEN** the user confirms the deletion
- **THEN** the trash is emptied
- **AND** the launcher hides

#### Scenario: Deletion is declined

- **WHEN** the user invokes empty trash and declines the confirmation
- **THEN** nothing is deleted
- **AND** the launcher returns to where it was

#### Scenario: Empty trash on macOS

- **WHEN** empty trash is confirmed on macOS
- **THEN** the user's Trash is emptied

#### Scenario: Empty trash on Windows

- **WHEN** empty trash is confirmed on Windows
- **THEN** the Recycle Bin is emptied

#### Scenario: Trash is already empty

- **WHEN** empty trash is invoked and there is nothing to delete
- **THEN** the command reports success without asking for confirmation

### Requirement: Quit application

Quit application SHALL offer the applications currently running as a list to
choose from, and SHALL ask the chosen one to quit in the ordinary way, so it can
prompt about unsaved work.

#### Scenario: Running applications are listed

- **WHEN** the user invokes quit application
- **THEN** the applications currently running are listed, each with its name and icon

#### Scenario: The list narrows as the user types

- **WHEN** the user types while the running applications are listed
- **THEN** the list narrows to the matching applications

#### Scenario: Chosen application quits

- **WHEN** the user chooses a running application
- **THEN** that application is asked to quit
- **AND** the launcher hides

#### Scenario: Application refuses to quit

- **WHEN** the chosen application does not quit, for example because it is prompting about unsaved work
- **THEN** the launcher does not force it
- **AND** no failure is reported, because the application is behaving correctly

#### Scenario: Dango is not offered

- **WHEN** the running applications are listed
- **THEN** Dango itself does not appear among them

#### Scenario: Application exits before it is chosen

- **WHEN** the chosen application has already exited
- **THEN** the failure is reported
- **AND** the launcher stays open
