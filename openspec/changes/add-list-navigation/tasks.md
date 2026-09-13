## 1. Take hover off the selection

- [x] 1.1 Set `disablePointerSelection={true}` on the `Command.Root` in
      `src/App.svelte` and in `src/lib/ProtocolView.svelte`, dropping both
      `pointerOwnsSelection()` calls; verify by arrowing to the third row,
      resting the pointer on another row, and pressing Enter - the arrowed row
      runs.
- [x] 1.2 Remove the `onpointermove` handler from `src/lib/ActionPanel.svelte`;
      verify hovering an action in an open panel no longer moves its selection.
- [x] 1.3 Narrow `src/lib/input.svelte.ts` to a paint-only `pointerActive()`
      flag with no say in selection; verify no caller reads it for selection and
      `npm run check` passes.

## 2. Keep exactly one row selected

- [x] 2.1 In `src/App.svelte`, replace the `?? results[0]` fallback with a
      derived `selectedId` over a writable `pickedId`, bound through
      `bind:value={() => selectedId, (id) => (pickedId = id)}`; verify by typing
      a query until the list is empty, deleting a character, and seeing the
      first row marked and Enter acting on it.
- [x] 2.2 Apply the same derived selection in `src/lib/ProtocolView.svelte`
      against the filtered `items`; verify by narrowing a pushed list view to
      nothing and widening it again - a row is always marked.
- [x] 2.5 Scroll the first row into view from a `bind:ref` on `Command.List`,
      working around the primitive skipping its scroll for that row; verify in a
      headless run that arrowing or holding Arrow Up from a scrolled position
      ends with the first row selected and fully visible (`scrollTop` 0).
- [x] 2.3 Take the primitive's selection writes only during a key press or a
      click, so its re-sort cannot drag the selection to the top, and clear the
      pick when the query changes; verified headlessly that a late results event
      for the same query keeps the picked row and that typing selects the first.
- [x] 2.4 Verified Arrow Up on the first row keeps that row selected and leaves
      the prompt's text cursor where it was, and Arrow Down on the last row
      stays on the last row.
- [x] 2.6 Prevent the default on a row's `mousedown` so clicking a row does not
      blur the prompt; verified the prompt keeps focus and accepts typing right
      after a click.

## 3. Give hover its own quieter state

- [x] 3.1 Change the row class in `src/App.svelte` and
      `src/lib/ProtocolView.svelte` to a hover fill guarded against the selected
      row (`[&:hover:not([data-selected])]:bg-muted/50` alongside
      `data-[selected]:bg-muted`); verify the selected row's appearance does not
      change when the pointer is over it.
- [x] 3.2 Give the action panel's entries the same hover fill, guarded against
      its selected entry; verify hovered and selected actions are told apart at
      a glance.
- [x] 3.4 Gate the hover fill behind `data-pointer` on the list container and
      the action panel, set from `pointerActive()`; verify in a headless run
      that hovering paints a row, a key press clears that paint, and moving the
      pointer brings it back.
- [x] 3.5 Move the list's vertical inset from `Command.Viewport` to a wrapper
      around `Command.List` so the gap is part of the frame; verify from
      screenshots that the gap at the top and bottom edges is identical when
      scrolled to the top, part way, and to the bottom.
- [x] 3.3 Check the hover fill in the dark theme, which is the only one the app
      can reach (`index.html` hard-codes `class="dark"`, and there is no theme
      switch); verified a hovered row paints `muted` at 50% against a selected
      row's full `muted`, so it is visible and clearly weaker. The light tokens
      in `app.css` stay unverified until something can select them.

## 4. Verify on both platforms

- [x] 4.1 On macOS, run through every scenario in
      `specs/list-navigation/spec.md` on all three surfaces: root search, a
      pushed list view, and the action panel. Covered by a headless sweep of the
      real frontend against a stubbed Tauri bridge, plus the author's own run in
      the launcher window.
- [ ] 4.2 On Windows, run the same pass; verify nothing differs, since the
      change has no platform branch.
- [ ] 4.3 Run `npm run check` and confirm CI is green before the change is
      considered done. `npm run check` and `npm run build` pass locally; CI is
      still to run.
