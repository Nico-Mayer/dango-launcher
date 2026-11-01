## REMOVED Requirements

### Requirement: The prompt stays typeable while the list holds the selection

**Reason**: Its scenario "Typing after clicking a row is not needed elsewhere"
is garbled and conflicts with "Clicking a row acts on that row", where a click
confirms the row. A MODIFIED block cannot drop a scenario, so the rule moves to
a new header without it.

**Migration**: See "The prompt stays typeable while a row is selected".

## ADDED Requirements

### Requirement: The prompt stays typeable while a row is selected

The prompt SHALL hold the keyboard focus the whole time the launcher is open,
except while a surface that owns the keyboard is up, such as the action panel.
Typing SHALL always reach the prompt, without the user having to click it or
arrow back to it, and having a selected row SHALL NOT take typing away from it.
While the action panel is open, typing SHALL reach the panel's filter input
instead, and SHALL reach the prompt again once the panel closes.

#### Scenario: Typing after arrowing

- **WHEN** the user arrows down several rows and then types a character
- **THEN** the character is inserted into the prompt and the query narrows

#### Scenario: Typing with the action panel open

- **WHEN** the action panel is open and the user types a character
- **THEN** the character goes to the panel's filter input and the prompt's query
  is unchanged

#### Scenario: Typing after the panel closes

- **WHEN** the user closes the action panel with Escape and types a character
- **THEN** the character is inserted into the prompt

