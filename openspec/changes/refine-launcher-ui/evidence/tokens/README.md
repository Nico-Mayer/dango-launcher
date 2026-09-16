# Stage 2 token foundation evidence

## Scope and environment

Tasks 2.1–2.3 complete. Same macOS arm64/Chromium environment as baseline;
720×400 CSS pixels, device scale factor 1. Browser-only evidence, not native proof.
Built-in colors intentionally retain stage-1 values. No theme loader, picker,
automatic mode selection, protocol change, production dependency, or new motion.

Three layers: `foundations.css` owns scales; `themes.css` owns built-in palette
values; `tokens.css` owns semantic/component roles and Tailwind compatibility aliases.
Root variables inherit into portals. App and existing components change presentation
classes only; refs, events, action ownership, DOM structure, and icon provider remain.
Result icon/tile roles are defined for stage 3.4; that task still owns their migration.
Existing status icons consume size tokens while retaining Lucide numeric size props
for unchanged `absoluteStrokeWidth` rendering.

## Validation

| Check | Result |
| --- | --- |
| Task 2.1 check/build/browser gate | Passed, 30/30 browser checks; 64px search, 56px rows, 40px footer, resolved selection color, transparent document. |
| Task 2.2 check/build/browser gate | Passed, 34/34 browser checks. |
| Final `npm run check` | 0 errors, 0 warnings. |
| Final `npm run build` | Passed, 727 modules; JS 112.76 kB (gzip 37.38 kB), CSS 27.34 kB (gzip 5.75 kB). |
| Final `npm run test:browser` | 47/47 passed, 6.7s. |
| `npm run test:browser -- --repeat-each=3` | 141/141 passed, 16.0s. |
| Svelte autofixer | No issues in App, ProtocolView, FormFields, ActionPanel, ResultRow, or TokenPortal. Existing effect synchronization advisories in first three retained; no script/event logic changed. |
| Production inspection | Only index.html and production JS/CSS; no dangoFixture, TokenPortal, data-token-portal, setTemplateInspection, or unstubbed-command marker. |
| `git diff --check` | Passed. No staged files or commits. |

New `tests/browser/theme.spec.ts` adds 19 checks:

- Root/list density, synchronous dark class, transparent html/body, resolved colors.
- Independent selection fill/text/edge, hover, focus, keycaps, controls, checkbox
  accent, field/carried errors, pending status/indicator, all six tint pairs.
- Root/list and current action panel share selected fill while hover/focus/busy
  remain independent. Primary/secondary/tertiary text and raised surfaces inherit.
- Foundation overrides affect typography, weights, row padding/radii, shell edges,
  overlay width/radius/elevation/layer, status icon size, and portal typography/radius.
- Root/list/form/detail/failure/long-actions render in both built-in palettes.

Initial gates caught an empty generated alias list and source-mismatched test
locators (footer div, pushed main, template label including help, banner-owned text).
Corrected without weakening assertions or changing production behavior. All final
gates above pass. Optional Pillow inspection unavailable; installed Playwright PNG
decoder used instead, without installing dependencies.

## Portal boundary

Supervisor approved minimal development-only `TokenPortal.svelte` probe, loaded
only with `?tokens=1`. It uses installed Bits `Portal` to place token consumers as
body children. Assertions prove root inheritance and independent busy-role values,
not production Popover focus/exit behavior or shared busy lifecycle. Actual portaled
action-panel and busy-surface override checks remain required in stages 4/5.
Probe adds no production entry or runtime configuration.

## Screenshots

Twelve PNGs: `dark-` and `light-` versions of root, list, form, detail, failure,
and long-actions. Each 720×400. Opened dark/light root, light form, and dark long
actions for visual inspection. PNG decoded RGBA comparison against stage-1 baseline:

| Dark scene | Differing pixels |
| --- | ---: |
| root | 0 |
| list | 0 |
| form | 0 |
| detail | 0 |
| failure | 0 |
| long-actions | 0 |

Existing faint metadata, overflowing long custom action panel, and missing populated
loading status remain visible, unchanged from baseline. Their fixes belong to later
assigned tasks; these screenshots do not claim final contrast, bounded Popover,
loading/accessibility, reduced-motion, native scaling, or activation-latency success.

## API references checked

- Tailwind v4 official theme documentation through Context7
  `/tailwindlabs/tailwindcss.com`: `@theme inline`, CSS variable inheritance,
  spacing/type/radius/shadow namespaces. Installed `tailwindcss/theme.css` used
  to retain exact default type stacks, scales, and overlay shadow.
- Bits Portal documentation through Context7 `/websites/bits-ui` and installed
  `bits-ui/dist/bits/utilities/portal/portal.svelte`: default body target,
  named `Portal` export, no replacement portal implementation.
