# Shared presentation extraction checkpoint

Tasks 3.4 and 3.5 complete; overall progress 10/27. No stage-4 action-panel behavior or stage-5 visual/loading behavior was introduced here.

## Verification

Environment: macOS host, Node v24.21.0, installed Playwright Chromium, 720×400 CSS pixels. Browser-only evidence; no native Windows/macOS verification claimed.

- `npm run check`: 0 errors, 0 warnings.
- `npm run build`: pass, 745 modules; CSS 27.80 kB and JS 125.43 kB (40.46 kB gzip).
- `npm run test:browser`: 60/60 pass after final whitespace cleanup. Covers existing navigation, delayed-result identity, resting-pointer suppression, first-row scrolling, typing focus, forms, failures, geometry, palette/token overrides, and the new icon checks.
- `npm run test:browser -- tests/browser/ResultRow.spec.ts tests/browser/theme.spec.ts`: 22/22 pass at task 3.4 checkpoint.
- `npx @sveltejs/mcp svelte-autofixer <file> --svelte-version 5`: all 22 touched/new Svelte components analyzed, zero issues. Raw output: `svelte-analysis.txt`. App and ProtocolView were reanalyzed after whitespace-only cleanup; same suggestions, no issues.
- `git diff --check`: pass. No staged files.
- Production bundle search for `dangoFixture`, `fixture-only`, `Clipboard preview`, and `fixture/application`: no matches.

Autofixer suggestions were inspected, not applied blindly: existing DOM scroll/focus effects and async IPC search/template inspection are side effects, not derived values. Button's bindable DOM ref intentionally uses `bind:this`, matching Bits' ref-forwarding pattern. These are not new orchestration changes.

## Icon evidence

`ResultRow.spec.ts` adds three checks (dark/light identities plus live token overrides). Existing `theme.spec.ts` checks every named tint pair. Screenshots here show both built-in palettes:

- `*-tints-first.png`, `*-tints-last.png`: all six extension tint identities across scrolled root results.
- `*-result-identities.png`: 32px identity tiles, 20px Lucide glyphs, highlighted match, contained application/favicon raster images, unknown named-icon placeholder. The fifth row continues below the scroll viewport by design.
- `*-clipboard-preview.png`: framed, cropped clipboard content and unknown named-icon fallback in a pushed list.

The 64×32 two-color raster is synthetic fixture data, used to make containment versus cropping visible. Tests assert loaded natural dimensions, object-fit, untinted image styles, border/radius dimensions, retained Lucide size/stroke-width attributes, and token overrides. Unknown names `icon:unregistered` and `icon:toString` produce no image or SVG; only the three valid file paths generate asset requests. Provider registry stays intact, with own-property lookup rejecting inherited object names.

Initial test setup failures were resolved: no global Node Buffer types exist in this project, so image fixtures use a static PNG with Playwright's file response; official Tauri `mockConvertFileSrc` was added because IPC mocking alone does not install file conversion. No production dependency added.

## Remaining boundaries

Actual Popover/Command portal integration, new busy surfaces, visual contrast refinement, reduced motion, and native/screen-reader/performance verification remain in later unchecked tasks. Stage-2 portal probe remains inheritance-only evidence. Existing palette contrast is not certified by this extraction checkpoint. Independent reviewer acceptance remains required.
