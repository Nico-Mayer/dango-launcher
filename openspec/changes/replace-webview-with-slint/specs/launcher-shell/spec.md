## ADDED Requirements

### Requirement: The launcher renders inside the Dango process

Dango SHALL draw the launcher, the tray menu, and every view inside its own
process. It SHALL NOT start a browser engine or any helper process to render
its interface.

#### Scenario: Process list on Windows

- **WHEN** Dango is running and the launcher has been shown and hidden
- **THEN** Task Manager lists one Dango process
- **AND** no WebView2 or other helper process belongs to it

#### Scenario: Process list on macOS

- **WHEN** Dango is running and the launcher has been shown and hidden
- **THEN** Activity Monitor lists one Dango process
- **AND** no WebKit or other helper process belongs to it
