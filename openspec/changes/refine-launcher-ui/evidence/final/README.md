# Final frontend regression: task 6.1

Environment: macOS 26.6.2 arm64 (Apple M2 Pro), Node v24.21.0, rustc 1.98.0,
Svelte 5.56, Bits UI 2.19.2, Playwright 1.63.0 Chromium. Browser evidence only;
native, screen-reader, and activation-latency checks (tasks 6.2 to 6.7) remain
open, with their state noted under each task in `tasks.md`.

## Commands

- `npm run check`: 683 files, 0 errors, 0 warnings.
- `npm run build`: pass, 747 modules; CSS 31.47 kB (6.57 kB gzip), JS 207.24 kB
  (64.19 kB gzip). Fixture code is not in the production entry.
- `npx playwright test --workers=4`: 155/155 pass, 0 skipped, 0 flaky. The two
  new stress checks below are included.
- `npx playwright test tests/browser/navigation.spec.ts --grep thirty --repeat-each=5`:
  10/10 pass.
- `npx @sveltejs/mcp svelte-autofixer <file>` over all 24 touched Svelte
  components and modules: 23 report zero issues (`svelte-analysis.txt`).
  `src/lib/ui/command.ts` reports a JavaScript parse error because the analyzer
  reads a plain TypeScript re-export module as JavaScript; `svelte-check` compiles
  the same file cleanly, so this is an analyzer limitation, not a code issue.
  Remaining suggestions on App and ProtocolView concern the intentional
  imperative focus/scroll/search effects and `bind:this` refs and are unchanged
  from the stage 4 and 5 reports.
- `git diff --check`: pass.

## Thirty same-view replacements per second

`navigation.spec.ts` now replaces the root result set and a pushed list thirty
times at 30 Hz after ArrowDown moved selection to the second row. For both
scenes the listbox and the selected row keep their DOM nodes, `data-selected`
and `aria-selected` never mutate, the search input keeps focus, and no CSS
animation or transition runs inside `main` other than the busy edge sweep.
The existing detail streaming check (`BusyStatus.spec.ts`) already covers thirty
detail replacements with one stable status node, one sweep animation, focus on
the latest content, and zero live-region mutations.

Finding while writing the check: the first run reported one running
`CSSTransition` on the footer Actions button. It was the button's 100ms colour
transition from disabled to enabled when the very first root results arrive,
which the test had started inside of. Replacements themselves trigger nothing;
the check now waits for a settled state before the burst. No code change.

## Diff inspection

- Visual literals: no hex, `rgb()`, `oklch()`, or pixel literals and no Tailwind
  arbitrary values remain in `src/lib/ui`, `src/lib/launcher`, feature components,
  or `App.svelte`; values resolve through `src/lib/theme/*.css` and `@theme inline`.
- Comments: four comment lines were added to modified files (two in
  `ActionPanel.svelte` on the Bits closing-boundary and deferred-dismiss adapters,
  two in `input.svelte.ts` on the synchronous `onValueChange` guard). Each records
  a primitive constraint documented in design.md. New components carry no comments.
- Protocol: `src-tauri/` and the generated protocol types are untouched. No
  `protocolVersion` or view shape change.
- Theme loader scope: no file reading, `theme.json`, `prefers-color-scheme`
  detection, or picker was added. The light palette is reachable only through
  the development class toggle.
- Dependencies: `@playwright/test` added as a dev dependency with a
  `test:browser` script; `tsconfig.json` includes `tests/` so the checks are
  type-checked. No production dependency added.

## Native macOS review findings

Two defects reported from the native macOS build after this stage, both fixed
in the shared components and covered by browser regressions:

- **Search focus ring.** The inset focus shadow added to the search input in
  stage 5.6 drew a blue frame across the header and was clipped by the shell's
  rounded top corners. Removed from `CommandInput.svelte` and the unused
  `--inset-shadow-focus` token deleted. The prompt holds focus for the whole
  session, so its caret marks focus; design.md decision 1 records the exemption.
  `visual-accessibility.spec.ts` now asserts the focused search input has no
  box-shadow or outline and keeps the 3:1 focus checks for fields, the trigger,
  and the action scope.
- **Row after a group heading clipped.** Bits 2.19.2 wraps every item in a
  `display: contents` node repeating its `data-value`, so its first-in-group
  branch matches every row and scrolls only the heading. The row after
  "Everything else" was selected but 3px below the scroller. The two per-view
  first-row `scrollTop` effects are replaced by one `MutationObserver` in
  `CommandList.svelte` that scrolls the selected row `nearest` and resets the
  first row to the top. New `navigation.spec.ts` check walks down and up across
  both headings and requires the selected row inside the scroller each step.

After the fixes: `npm run check` 0 errors, `npm run build` pass, browser suite
156/156, Svelte analysis on the four touched components reports no issues (the
list wrapper's effect suggestion is the intentional DOM observer).

**Action panel frame (same defect, second surface).** The panel's command scope
carried `focus-visible:outline-focus`, an inset ring framing the whole overlay,
for the same contrast check that added the search ring. Removed; the selected
row plus `aria-activedescendant` is the focus indicator, as in the main list.
The unused `outline-focus` utility and the unused `Button.svelte` wrapper (whose
`outline-focus-ring` class set only a colour and drew nothing) were deleted.
Discrete controls keep their rings: fields recolour their border, the checkbox
and footer trigger use an outer offset ring. The detail region already had no
ring. The visual-system spec and design decision 1 now state that whole surfaces
never carry a ring. Screenshots for root, refreshing-list, and actions in
`../refinement/final/` were refreshed from this run; after the change:
`npm run check` 0 errors, build pass, browser suite 156/156.
