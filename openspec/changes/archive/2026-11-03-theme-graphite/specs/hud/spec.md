# Spec Delta

## MODIFIED Requirements

### Requirement: The HUD is a pill with a dot and one line

The HUD SHALL look like a toast on its own: a success dot or a failure dot, as
the toast uses, followed by one line of text, in a rounded pill that fits its
text up to the toast's width. Every colour, size, and radius SHALL come from the
theme, in dark and light. Its text SHALL follow the same copy rules as a toast's.
Assistive technology SHALL be told the text and whether it reports a failure.

#### Scenario: A failure HUD

- **WHEN** the HUD reports a failure
- **THEN** it shows the danger dot and the failure's one sentence

#### Scenario: A success HUD

- **WHEN** the HUD reports a copy
- **THEN** it shows a dot in the done role and "Copied to clipboard"

#### Scenario: The HUD follows the system theme

- **WHEN** the operating system switches between dark and light
- **THEN** the next HUD uses the matching theme values
