# Browser fixtures

Run `npm run dev`, then open `http://127.0.0.1:1420/tests/fixtures/?scene=root`.
Scenes: `root`, `list`, `empty-list`, `loading-list`, `refreshing-list`, `detail`,
`form`, `template`, `failure`, `long-actions`.

Fixtures mount the real App with the official Tauri `mockIPC` event implementation.
Commands are recorded, never sent to Rust. Unknown commands fail rather than reaching
native APIs. Template inspection returns fixed sample data; it does not implement the
backend template engine. All displayed data, including the password value, is synthetic.

Browser console controls:

```js
await dangoFixture.show("detail");
dangoFixture.stream(); // 30 full-tree replacements, then loading ends
await dangoFixture.show("long-actions"); // Ctrl+K or Cmd+K opens actions
await dangoFixture.emit("dango://failed", "Couldn't paste. Try again.");
await dangoFixture.emit("dango://reset");
dangoFixture.calls;
```

`replace(tree)` sends another tree for the current invocation; `results(query, items,
complete)` delivers root results, including deliberately stale/delayed results. Streaming
stops on cancellation, dismissal, scene changes, completion, or page exit.

## Token preview

Append `&tokens=1` to a scene URL to mount a development-only Bits Portal probe.
It checks document-root token inheritance, not production action-panel behavior.
Toggle `document.documentElement.classList.toggle("dark")` in browser console to
inspect the built-in light palette. Override `--dango-*` root style properties to
inspect independent roles; neither operation adds application theme configuration.

`dangoFixture.setTemplateInspection({ arguments: [], error: "Sample error" })`
sets the next stubbed template inspection response. Edit the template to receive it.
This stays fixture data, not a frontend template parser.

Token checks and both-palette screenshots: `tests/browser/theme.spec.ts` and
`openspec/changes/refine-launcher-ui/evidence/tokens/`.

## Regression checks

```sh
npm ci
npx playwright install chromium
npm run test:browser
npm run check
npm run build
```

Playwright starts its own Vite server on port 1422. Tests use isolated contexts at
720×400 CSS pixels. Runtime screenshots/traces go to ignored `test-results/`.
Fixture smoke tests capture screenshots without asserting pixel equality, so later
intentional visual changes do not require overwriting the original evidence.

Original screenshots and outcomes live in
`openspec/changes/refine-launcher-ui/evidence/baseline/`.

`tests/browser/ResultRow.spec.ts` supplies application/favicons and clipboard previews
through intercepted asset requests using the synthetic 64×32 `identity.png`. The
fixture uses Tauri's `mockConvertFileSrc("windows")` to expose routable asset URLs;
this is browser rendering coverage, not native Windows evidence. Both-palette icon
screenshots and extraction checks live in
`openspec/changes/refine-launcher-ui/evidence/extraction/`.

`tests/browser/ActionPanel.spec.ts` proves the isolated composition through
`/tests/fixtures/actions.html`. `ActionPanel.integration.spec.ts` exercises the real
root/list/form/detail panels, focus and dismissal boundaries, both chords, long
lists, and live portal token overrides. Stage-4 screenshots and outcomes live in
`openspec/changes/refine-launcher-ui/evidence/actions/`. Production panel motion is
still off at that checkpoint; test-only held exit presence is not a shipped effect.

The fixture HTML is a separate Vite development entry, not imported by `src/main.ts`
or configured as a production build input. Its module also rejects non-development
execution. Inspect `dist/` after building: only the normal index and bundled assets
should exist; neither fixture API nor sample data should appear there.

## Motion, busy state, and compact accessibility

`refinement.spec.ts` and both modes of `ActionPanel.integration.spec.ts` exercise
production 140ms/90ms panel animations, immediate keyboard feedback, repeated
input, outside dismissal and resets. `BusyStatus.spec.ts` checks declared busy
lifecycles, stable status/sweep identity, suppressed chunk announcements, live
reduced motion and cleanup. Document hiding is simulated through the standard
visibility event; this is not a native hidden-window performance trace.

`visual-accessibility.spec.ts` checks composited text contrast and actual focus
indicators in both palettes, long content, live overrides, and 720×400/480×300
viewports. JSON reporter attachments contain contrast measurements. Final browser
screenshots and outcomes are under
`openspec/changes/refine-launcher-ui/evidence/refinement/final/`.
