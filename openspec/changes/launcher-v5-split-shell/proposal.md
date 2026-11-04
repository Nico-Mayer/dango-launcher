## Why

The launcher looks like a Raycast clone: one card with a search field on top,
a list, and a footer. Design system v5 (`Launcher Component System v5.dc.html`)
gives Dango its own look. The main differences are a search pill that stands
apart from the results and a launcher that opens as that pill alone.

## What Changes

- The SearchBar becomes a pill, and the view and ActionBar sit in a separate
  card below it, with a transparent gap between the two. Both are in the one
  launcher window. **BREAKING** for the shell sizes in `design-system`.
- Parts get reshaped to match v5: rounded inset selection in lists, the
  primary action in the ActionBar shown as a chip, the back control in a
  circle, the filter dropdown in the SearchBar shown as a pill, and larger
  radii. Colours, type faces and themes stay as they are.
- The pill and the card cast a shadow where the renderer can draw one.
- **BREAKING** The launcher opens as the bare pill with an empty query. Results
  appear on the first keystroke. Clearing the query goes back to the bare pill.
- When a provider has a pinned item, the card opens by itself with only the
  "Needs attention" section.
- Arrow Down on an empty pill opens today's empty-query list ("Needs
  attention", "Suggestions", "Everything else"), so recent items stay easy to
  find.
- Escape gets one more stage: in the list opened by Arrow Down, Escape closes
  the list and goes back to the pill. The next Escape hides the launcher.
- While only the pill shows, a toast appears as a small capsule under the pill
  instead of in the ActionBar.

## Non-goals

- No glass or background blur. Slint cannot blur what is behind an element,
  and the OS blur covers the whole window, not two rounded shapes.
- No new themes, colours, or type faces. v5 uses the same herdr roles and
  Geist faces Dango already has.
- No change to how typed queries are ranked: frecency and learned inputs stay
  as they are.
- No window resizing on macOS. It stays full height, as today.
- No change to the HUD, the action panel's contents, or the view protocol.
- No new design for detail, form, grid, or empty views beyond what the shared
  parts bring.

## Platforms

Windows and macOS. The window margin for shadows, the bare-pill height and
the transparent areas touch window placement on both, so each platform gets
its own hand checks. Linux is out of scope for the project.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `design-system`: the shell becomes a pill and a separate card; the sizes
  change; the toast gets a capsule form under the pill; parts take the v5
  shapes.
- `root-search`: an empty query shows no results unless the user asks for
  them with Arrow Down; pinned items open the card on their own.
- `launcher-shell`: the window height follows the bare pill on Windows; the
  launcher has a shadow margin around the visible shell; Escape gets the extra
  stage.
- `list-navigation`: Arrow Down on an empty prompt opens the empty-query list.

## Impact

- `ui/theme.slint`, `ui/shell/*` (launcher window, SearchBar, ActionBar,
  toast), `ui/views/list.slint` and `grid.slint` (row shape).
- `src/ui/state.rs` and `snapshot.rs` (open or closed list at root, Escape
  stage, bindings), `src/slint_shell/render/` (height, shell), and window
  placement in `src/platform/windows` and `src/platform/macos` (shadow
  margin).
- UI tests and state machine tests for the empty query, Escape, and heights.
- No dependency changes. No manifest or view protocol change.
