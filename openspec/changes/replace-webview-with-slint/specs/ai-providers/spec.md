## RENAMED Requirements

- FROM: `### Requirement: Requests are made from the application, never the webview`
- TO: `### Requirement: Requests are made by the application core, never the UI layer`

## MODIFIED Requirements

### Requirement: Requests are made by the application core, never the UI layer

Every request to a provider SHALL be made by the application core. No key and
no request SHALL be handed to the code that draws the interface, and no
provider endpoint SHALL be called from it.

#### Scenario: The webview never holds a key

- **WHEN** an AI command runs
- **THEN** the key and the request stay inside the application core
- **AND** the interface receives only the text of the answer
