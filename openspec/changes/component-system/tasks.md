## 1. Theme, fonts, and atoms

- [x] 1.1 Add `ui/theme.slint` with the `Theme` global from design D5, deriving dark from std-widgets `Palette.color-scheme`; verify with a UI test in `src/slint_shell/tests.rs` that forces each scheme and reads the surface and muted roles
- [ ] 1.2 Add Geist 400/500/600 and Geist Mono 400/500 to `ui/fonts/` with their OFL licence file, imported in `ui/theme.slint`; verify `cargo test` passes and a UI test reads `default-font-family` as Geist
- [x] 1.3 Add `ui/atoms/` Icon, Kbd, Tag, Accessory, and Dot built only on `Theme`; verify UI tests find each by `accessible-label` and that a Kbd renders the platform chord from `keys.rs`
- [x] 1.4 Move `ui/app.slint` from `Palette` to `Theme` and delete `Palette`; verify `cargo test` and `hk check` pass and `rg Palette ui` finds only the std-widgets import
- [x] 1.5 Remove tints: `Manifest.tint`, `ResultItem.tint`, `ExtensionHost::tints`, `tint_index`, `IconTile` tint, and the built-in manifests' tint fields; verify `cargo test` passes and `rg -w tint src ui` is empty
- [x] 1.6 Remove the development marker: `Snapshot.development`, the dev badge, the frame colour, and the tray's development tooltip; verify `cargo test` passes and `rg -i "development build" src ui` is empty

## 2. View protocol version 2

- [x] 2.1 Move `src/protocol.rs` to version 2 per design D6 and reject version 1; verify a unit test that a version 1 tree yields the unsupported-view state
- [x] 2.2 Add item sections, section titles and subtitles, accessories, and keywords; verify reducer tests for section order, hidden empty sections when filtering, Arrow Down crossing a section, and a keyword match
- [x] 2.3 Add action `section` and `style`, ordering danger actions last; verify a reducer test with "Pin entry" and "Delete entry" under "Manage"
- [x] 2.4 Add `EmptyState.icon` and `actions`, with Enter running the first; verify a reducer test that Enter on an empty view with "Search the web" emits its action
- [x] 2.5 Add `ActionOutcome::Toast` and the reducer toast state with `Effect::ExpireToast` and `Event::ToastExpired` per design D7; verify reducer tests for success expiry, a stale expiry ignored, and a failure cleared by a key
- [x] 2.6 Move `snapshot.failure` to failure toasts and running and loading status to busy toasts; verify the existing failure and status tests in `src/ui/tests.rs` pass against the toast
- [x] 2.7 Move every built-in extension to version 2 views; verify `cargo test` passes

## 3. Shell: globals, SearchBar, ActionBar, toast

- [x] 3.1 Split `ui/` into `theme`, `atoms/`, `shell/`, `views/`, and `app.slint` per design D3, with the globals from design D2; verify `hk check` and `cargo test` pass
- [x] 3.2 Split `render.rs` into `render/` with one writer per global; verify every test in `src/slint_shell/tests.rs` passes
- [x] 3.3 Build the SearchBar on `TextInput` with placeholder, "Go back" control, static loading bar, and title mode for detail and form; verify UI tests for the back control popping a view, the loading bar under a loading view, and a form showing its title
- [x] 3.4 Build the ActionBar: context (extension name and icon at root, command title and icon in a view), primary label with Enter Kbd, and "Actions" with the panel chord only when there is more than one action; verify UI tests for "Applications" with "Open", "Clipboard History" as context, and a single-action row
- [x] 3.5 Show toasts in the ActionBar's left slot with a static dot per style and accessible text; verify UI tests for each style and that typing reaches the SearchBar while a toast shows
- [x] 3.6 Delete the failure banner and the error screen kind, showing an unsupported view as an empty view with "This view needs a newer version of Dango" and "Press Escape to go back"; verify a UI test with a version 1 tree
- [x] 3.7 Cut the failure strings listed in design D12 and every other failure string in `src/` to one sentence that fits the toast; verify `cargo test` and a UI test that the longest failure string is not elided at 720 logical pixels

## 4. List view

- [x] 4.1 Render section headings with optional subtitle and rows at 40 logical pixels with a 20 pixel icon, up to three accessories, selection and hover from `Theme`; verify UI tests for row height, a fourth accessory dropped, and hover weaker than selection
- [x] 4.2 Update `reveal` for the new row and heading heights; verify the existing "arrowing back to the top shows the heading" UI test passes
- [x] 4.3 Build the EmptyView with icon, title, and hint, centred; verify a UI test for "No results for" with the query
- [x] 4.4 Build the list detail pane at two fifths and three fifths; verify a UI test that Arrow Down changes the pane's content

## 5. Action panel

- [x] 5.1 Restyle the panel with a shadow, section headings, and danger rows in the danger role; verify UI tests for the "Manage" section and the danger row's position
- [x] 5.2 Add the panel filter input with "Search actions" as placeholder and "No matching actions" when empty, with `Focus::Panel` placing the cursor in it; verify reducer and UI tests that typing "co" narrows the actions and leaves the query, and that Escape returns typing to the SearchBar
- [x] 5.3 Verify the overlay rules in UI tests: confirming in the panel runs one action and not the row below, and closing over a detail view gives it its shortcuts back

## 6. Detail view

- [x] 6.1 Render the detail body with `StyledText` from markdown and metadata as label and value pairs below it; verify a UI test with "Source" and "Copied" in order
- [x] 6.2 Use the same detail component in the list's pane; verify the 4.4 test still passes

## 7. Form view

- [x] 7.1 Build TextField, TextArea, Checkbox, Dropdown, and Separator atoms on primitives with an accent focus border and accessible role, label, value, and description; verify UI tests that each is found by label and that an error is the field's description
- [x] 7.2 Lay out the form as a label column and a control column with Tab and Shift+Tab order; verify a UI test that Tab moves the accent border
- [x] 7.3 Add the `textarea` and `dropdown` field kinds and the separator to the form protocol; verify reducer tests that a dropdown submits the chosen identifier and a text area keeps its line break
- [x] 7.4 Move the snippet and quicklink forms to the new layout; verify the existing snippet form tests pass

## 8. Grid view

- [x] 8.1 Add `GridView` with columns 4 to 8, aspect, and image, colour, or glyph content, rejecting other column counts with "This view can't be shown"; verify reducer tests for a two-column grid and a valid five-column grid
- [x] 8.2 Add grid navigation to the reducer per design D9; verify reducer tests for the scenarios in `list-navigation` "A grid moves in four directions"
- [x] 8.3 Render tiles in a `ScrollView` with computed positions, titles below, and the selected tile outlined in accent; verify a UI test for twelve tiles in five columns and that Arrow Left does not move the SearchBar's text cursor

## 9. Filter dropdown

- [x] 9.1 Add the view's filter to the reducer and the "Filter" section in the action panel; verify a reducer test that confirming "Text only" emits the filter effect and closes the panel
- [x] 9.2 Show the selected filter at the SearchBar's right end with "Filter results" as label, opening a popup on click; verify a UI test that choosing "Text only" keeps the query and shows "Text only"

## 10. Window height

- [ ] 10.1 Compute the launcher height in the renderer from row, heading, and tile heights, capped at 480, full height for empty, detail, and form views, and send it through `Shell::set_launcher_height`; verify a UI test with the recording shell for three results and for forty
- [ ] 10.2 Windows: implement the resize with `SWP_NOMOVE` and centre with the maximum height in `position_on_active_display`; verify `cargo clippy -- -D warnings` passes on Windows
- [ ] 10.3 macOS: implement the resize with `setFrame:display:` keeping the top edge, and centre with the maximum height; verify `cargo clippy -- -D warnings` passes on macOS
- [ ] 10.4 Windows manual check: type so the result count changes on each key; the SearchBar does not move and nothing flickers
- [ ] 10.5 macOS manual check: type so the result count changes on each key; the SearchBar does not move and nothing flickers

## 11. Operating system theme

- [ ] 11.1 Windows manual check: switch the app mode between dark and light while Dango runs and show the launcher after each switch; it uses the matching theme
- [ ] 11.2 macOS manual check: switch the appearance between dark and light while Dango runs and show the launcher after each switch; it uses the matching theme

## 12. Docs

- [ ] 12.1 Rewrite the UI parts of `AGENTS.md`: the `Theme` global, the `ui/` layout, own inputs on `TextInput` instead of Fluent and std-widgets first, the globals, and the macOS resize trap from design D8; verify `rg -n "Fluent|Palette|std-widgets.slint first" AGENTS.md` finds nothing outdated
- [ ] 12.2 Update the design rule in `openspec/config.yaml` to name the design-system component each surface uses instead of a std-widgets widget; verify `openspec validate component-system --strict` passes
