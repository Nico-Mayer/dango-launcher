## MODIFIED Requirements

### Requirement: One invocation at a time

Invoking a command while another is running SHALL supersede the first. The
launcher SHALL never show output from a superseded invocation, with one
exception: a no-view command's success message or failure that arrives after
it was superseded SHALL still be reported, in the notice or ticker, without
hiding the launcher, ending the newer command, or replacing its view.

#### Scenario: A second command supersedes the first

- **WHEN** the user invokes a command while an earlier one is still running
- **THEN** the earlier invocation's output is discarded
- **AND** only the newer command's output is shown

#### Scenario: A superseded no-view command still reports its message

- **WHEN** a no-view command finishes with "Brightness set to 70%" after a newer invocation superseded it
- **THEN** the notice reads "Brightness set to 70%"
- **AND** the newer invocation keeps running and its output is still shown
