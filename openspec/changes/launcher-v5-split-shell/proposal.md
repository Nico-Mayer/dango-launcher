## Why

The launcher looks like a Raycast clone: one card with a search field on top,
a list, and a footer. Design system v5 (`Launcher Component System v5.dc.html`)
gives Dango its own look. The main differences are a search pill that stands
apart from the results and a launcher that opens as that pill alone.

## What Changes

- The SearchBar becomes a pill, and the view and ActionBar sit in a separate
  card below it, with a transparent gap between the two. Both are in the one
  launcher window. **BREAKING** for the shell sizes in `design-system`.
- The card is as tall as its content on both platforms. Only Windows also
  resizes the window, as today.
- Parts get reshaped to match v5: the primary action in the ActionBar shown as
  a chip, the back control in a circle, the filter dropdown in the SearchBar
  shown as a pill. Colours, type faces and themes stay as they are.
- **BREAKING** The launcher opens as the bare pill with an empty query. Results
  appear on the first keystroke. Clearing the query goes back to the bare pill.
- When a provider has a pinned item, the card opens by itself with only the
  "Needs attention" section.
- Arrow Down on an empty pill opens today's empty-query list ("Needs
  attention", "Suggestions", "Everything else"), so recent items stay easy to
  find.
- Escape gets one more stage: in the list opened by Arrow Down, Escape closes
  the list and goes back to the pill. The next Escape hides the launcher.
- While only the pill shows, a toast appears at the pill's right end instead
  of in the ActionBar.
- A click in the gap between pill and card counts as a click outside the
  launcher.
- After the build, the v5 design file is updated to match the app, and the
  older design files are deleted, so the design can go back into Claude
  Design.

## Non-goals

- No glass or background blur. Slint cannot blur what is behind an element,
  and the OS blur covers the whole window, not two rounded shapes.
- No shadows in this change. Slint's software renderer cannot draw them, and
  they need a window margin and placement changes on both platforms. A
  follow-up adds them once the hand check shows which renderer runs.
- No renaming of "toast" or "HUD". That is a separate change.
- No new themes, colours, or type faces. v5 uses the same herdr roles and
  Geist faces Dango already has.
- No change to how typed queries are ranked: frecency and learned inputs stay
  as they are.
- No window resizing on macOS. It stays full height, as today.
- No change to the HUD, the action panel's contents, or the view protocol.

## Platforms

Windows and macOS. The shell, state machine and height maths are shared, and
neither platform's placement code changes. Each platform gets its own hand
check, because the transparent areas behave differently: Windows resizes the
window, macOS keeps a full-height window with a transparent area below the
card. Linux is out of scope for the project.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `design-system`: the shell becomes a pill and a separate card; the sizes
  change; the toast can show at the pill's end; parts take the v5 shapes.
- `root-search`: an empty query shows no results unless the user asks for
  them with Arrow Down; pinned items open the card on their own.
- `launcher-shell`: the card follows its content on both platforms; Escape gets
  the extra stage; a click in the gap is a click outside; the reset shows the
  closed root view.
- `list-navigation`: Arrow Down on an empty prompt opens the empty-query list.

## Impact

- `ui/theme.slint`, `ui/shell/*` (launcher window, SearchBar, ActionBar).
- `src/ui/state.rs` and `snapshot.rs` (closed root view, Escape stage,
  Arrow Down), `src/slint_shell/render/` (height, shell), `src/app/` (click in
  the gap is an interrupting hide, like the hotkey).
- Test helpers in `src/ui/tests.rs` and `src/slint_shell/tests.rs`.
- The design files in the author's Downloads folder (outside the repo).
- No dependency changes. No manifest or view protocol change. No platform code.
