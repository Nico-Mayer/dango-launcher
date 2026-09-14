## 1. Manifest carries a tint

- [x] 1.1 Add `tint: Option<String>` to `Manifest` in
      `src-tauri/src/extension/manifest.rs`, defaulted by serde, and verify
      `cargo test -p dango extension::manifest` still passes with every existing
      manifest construction updated
- [x] 1.2 Declare the tint on the six built-in manifests per the table in
      design.md (`dango.system` red, `dango.clipboard` amber, `dango.snippets`
      green, `dango.window-management` blue, `dango.ai` purple,
      `dango.quicklinks` pink), leaving `dango.applications` untinted, and verify
      `cargo test` passes
- [x] 1.3 Add a test listing the palette names and asserting every built-in
      manifest's tint is one of them, and verify it fails when a name is
      misspelled

## 2. The tint reaches the frontend

- [x] 2.1 Build the `extension id -> tint` map from the loaded manifests at
      startup in `src-tauri/src/lib.rs` and manage it as state, and verify the
      app still starts and search returns results
- [x] 2.2 Add `tint` to the `ResultItem` DTO, filled from that map in the
      `search` streaming loop, and verify a result from a tinted extension
      arrives with its tint in the `dango://results` payload while an
      applications result arrives with none

## 3. The tint is drawn

- [x] 3.1 Add the six light and dark token pairs to `src/app.css` following the
      existing `accent` formula, and verify `npm run build` emits the
      `bg-tint-*` and `text-tint-*-foreground` classes
- [x] 3.2 Add the tint registry beside `src/lib/Icon.svelte` mapping a name to
      its literal class pair and an unknown name to nothing, and verify
      `npm run check` passes
- [x] 3.3 Apply the tint in the named-icon branch of
      `src/lib/ResultRow.svelte` and pass it from the root list in
      `src/App.svelte`, and verify the root list shows tinted squares behind
      built-in command icons while application rows and pushed-view rows are
      unchanged

## 4. Verification

- [x] 4.1 Verify on macOS: open the launcher, confirm commands from different
      built-ins are separable at a glance, application icons carry no tinted
      square, and a pushed view's rows (clipboard history, Quit Application)
      look as they did before
- [ ] 4.2 Verify on Windows: the same check, including that an untinted
      extension's results and the placeholder square for an app whose icon could
      not be extracted are unchanged
- [ ] 4.3 Verify activation is still within budget with `DANGO_MEASURE=1` on a
      release build on both platforms, since every root row now paints one more
      filled element

Carried forward, unverified: 4.2 and 4.3. macOS was driven through a running
`tauri dev` session and the tints read as intended there. Windows has not been
opened since the change, and no release build was timed on either platform, so
the extra filled element per row is unmeasured.
