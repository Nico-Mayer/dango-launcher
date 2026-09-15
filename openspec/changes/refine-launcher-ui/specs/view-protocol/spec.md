## MODIFIED Requirements

### Requirement: Loading and empty states

A view SHALL be able to signal that it is loading and to declare what is shown when it has no content.

Every list or detail view marked loading SHALL show a busy indicator and status text, whether or not it already contains content. Entering the loading state SHALL NOT itself clear, replace, dim into unreadability, or block interaction with content supplied by the command. The indicator SHALL reflect the declared loading state, not infer completion from incoming text or elapsed time, and SHALL NOT claim a numerical progress value when none is provided.

Busy state SHALL be exposed to assistive technology. Status announcements SHALL describe state changes without announcing every streamed replacement. Decorative animation SHALL not be the only indication of work.

#### Scenario: Loading is visible

- **WHEN** a command emits a view marked as loading
- **THEN** a loading indicator is shown without clearing any content already displayed

#### Scenario: Empty state is shown

- **WHEN** a list view contains no items and is not loading
- **THEN** its declared empty state is shown

#### Scenario: Empty list is loading

- **WHEN** a command emits a loading list with no items
- **THEN** the list shows "Loading…" with a busy indicator rather than its completed empty state

#### Scenario: Populated list is refreshing

- **WHEN** a command emits a loading list with existing items
- **THEN** those items remain visible and selectable, and "Loading…" is shown with a busy indicator
- **AND** the existing item-identity rules continue to preserve selection

#### Scenario: Detail content is streaming

- **WHEN** a command repeatedly replaces a detail view marked loading
- **THEN** the supplied content remains visible alongside "Loading…" and a busy indicator
- **AND** the indicator stays active across replacements without replaying entrances or repeatedly announcing the same status

#### Scenario: Loading finishes

- **WHEN** the command emits a replacement with loading set to false
- **THEN** its content remains visible and the busy indicator and loading status are removed
- **AND** assistive technology is no longer told that the view is busy

#### Scenario: Loading feedback is abandoned

- **WHEN** the user leaves a loading view or hides the launcher
- **THEN** that view's activity animation stops
- **AND** a late update from an abandoned invocation does not restore its loading feedback

#### Scenario: Reduced motion preserves loading feedback

- **WHEN** a list or detail view is loading with reduced motion enabled
- **THEN** a static indicator and "Loading…" communicate the same busy state without a sweep, shimmer, or pulse
