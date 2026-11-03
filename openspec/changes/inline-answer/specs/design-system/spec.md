# Spec Delta

## MODIFIED Requirements

### Requirement: Every screen is one shell around one view

The launcher SHALL show, from top to bottom, the SearchBar, exactly one view,
and the ActionBar. The view is a list, a grid, a detail, a form, a progress
view, or an empty view. In root search, an answer card MAY sit between the
SearchBar and the results. Overlays SHALL float above these regions and SHALL
NOT move them.

The launcher SHALL be 720 logical pixels wide. The SearchBar SHALL be 64
logical pixels high, the ActionBar 44, and a list row 44.

#### Scenario: Root search

- **WHEN** the launcher opens at root
- **THEN** it shows the SearchBar, a list of results, and the ActionBar

#### Scenario: Sizes at default text scale

- **WHEN** root search is shown at the default text scale
- **THEN** the launcher is 720 logical pixels wide, the SearchBar is 64 high,
  each result row is 44 high, and the ActionBar is 44 high

#### Scenario: Root search with an answer

- **WHEN** the user types "2+2" at root
- **THEN** the launcher shows the SearchBar, the answer card, the results, and
  the ActionBar, from top to bottom
