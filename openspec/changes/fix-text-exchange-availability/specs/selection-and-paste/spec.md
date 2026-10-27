## MODIFIED Requirements

### Requirement: The macOS permission is a visible state

On macOS both reading the selection and inserting text require the Accessibility
permission. Dango SHALL detect that it is missing and SHALL explain it, rather
than failing silently. Whether the permission was granted before or after Dango
started SHALL make no difference.

#### Scenario: Acting without the permission on macOS

- **WHEN** the user invokes something that reads the selection or inserts text and the permission has not been granted
- **THEN** nothing is sent to the other application
- **AND** the launcher explains that the permission is needed and offers to request it

#### Scenario: Requesting the permission on macOS

- **WHEN** the user chooses to request the permission
- **THEN** the system's permission prompt is shown
- **AND** no prompt appears at any other time

#### Scenario: The permission is granted on macOS

- **WHEN** the permission has been granted
- **THEN** reading the selection and inserting text work without any further prompting

#### Scenario: The permission is granted after Dango started on macOS

- **WHEN** Dango was started without the permission and the user grants it while Dango is running
- **THEN** the next insertion or selection read works without restarting Dango

#### Scenario: Windows needs no permission

- **WHEN** the user invokes something that reads the selection or inserts text on Windows
- **THEN** it works with no permission step
- **AND** no message about a permission is ever shown

### Requirement: Failures are reported, never silent

An insertion or a read that cannot be completed SHALL leave a message the user
can act on, and SHALL leave the clipboard as it found it. The message SHALL name
the actual cause: a missing permission SHALL be reported only when the
permission is missing.

The text path SHALL NOT depend on the clipboard history. With the history
unavailable, reading the selection and inserting text SHALL still work.

#### Scenario: The target window is elevated on Windows

- **WHEN** the frontmost window belongs to an elevated process
- **THEN** the insertion fails with a message saying so
- **AND** the clipboard is unchanged

#### Scenario: There is no application to insert into

- **WHEN** there is no previous application to return to
- **THEN** the insertion fails with a message saying so
- **AND** the clipboard is unchanged

#### Scenario: The text path could not be set up on Windows

- **WHEN** Dango could not set up the text path at startup on Windows and the user invokes something that reads the selection or inserts text
- **THEN** the launcher says Dango can't reach other apps and names `dango.log`
- **AND** the message does not mention a permission

#### Scenario: The text path could not be set up on macOS

- **WHEN** Dango could not set up the text path at startup on macOS and the user invokes something that reads the selection or inserts text
- **THEN** the launcher says Dango can't reach other apps and names `dango.log`
- **AND** the message does not mention a permission

#### Scenario: The tray reports a text path that could not be set up

- **WHEN** Dango could not set up the text path at startup
- **THEN** a disabled tray item says pasting is off and names `dango.log`

#### Scenario: The clipboard history is unavailable

- **WHEN** the database could not be opened at startup
- **THEN** clipboard history is off
- **AND** reading the selection and inserting text still work, through AI commands and snippets
