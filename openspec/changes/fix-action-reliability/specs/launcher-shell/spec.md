## MODIFIED Requirements

### Requirement: Reset to root state on hide

Because the window is reused, hiding the launcher SHALL signal the frontend to
return to its initial state. The next activation SHALL present the launcher as
if it had just started.

A failure message the user has not seen SHALL be the one exception. An action
that hides the launcher as part of its work can only report a failure once the
window is gone, so a failure SHALL survive the reset and be shown on the next
activation. It SHALL be cleared as soon as the user types, runs another action,
or presses Escape.

#### Scenario: State does not leak between invocations

- **WHEN** the user types into the prompt, hides the launcher, and shows it again
- **THEN** the prompt is empty

#### Scenario: A failure from a dismissing action is shown afterwards

- **WHEN** an action hides the launcher and then fails
- **THEN** the next activation shows that failure's message

#### Scenario: The user clears a carried failure

- **WHEN** a carried failure is on screen and the user types, runs an action, or
  presses Escape
- **THEN** the message goes away
