# Spec Delta

## MODIFIED Requirements

### Requirement: A task's result while hidden shows on the next open

A task that finishes, fails, or comes to need input while the launcher is
hidden SHALL show that in the HUD at once, as a success or a failure, and as a
toast the next time the launcher opens: a success toast when it finished or
needs input, and a failure toast with its failure sentence when it failed.
When the next open shows the task's own view, the view reports it and no toast
is shown. When the result offers "Show answer" and the user opens the launcher
while its HUD still shows, the answer's view SHALL open and no toast is shown.

#### Scenario: A download fails while hidden

- **WHEN** an update download fails while the launcher is hidden and the user
  then opens the launcher
- **THEN** the HUD showed the failure when it happened, and a failure toast
  reads "Couldn't download the update. Check your connection and try again."

#### Scenario: An update is ready while hidden

- **WHEN** an update finishes downloading while the launcher is hidden and the
  user then opens the launcher
- **THEN** a success HUD read "Dango 0.5.0 is ready to restart" when it
  happened, and a success toast reads the same

#### Scenario: Reopening onto the task's own view

- **WHEN** an update fails while the launcher is held on its progress view, and
  the user opens the launcher within the grace period
- **THEN** the progress view shows the failure and no failure toast is shown

#### Scenario: Opening while an answer's HUD shows

- **WHEN** an AI answer finishes while the launcher is hidden, and the user
  opens the launcher while its HUD still shows
- **THEN** the launcher opens on the answer and no toast is shown

#### Scenario: Opening after an answer's HUD is gone

- **WHEN** an AI answer finishes while the launcher is hidden, and the user
  opens the launcher after its HUD is gone
- **THEN** a success toast reads "Answer ready: " and the command's title, with
  "Show answer"
