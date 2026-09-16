# Bits action panel checkpoint

Tasks 4.1–4.4 complete; overall progress 14/27. This continuation finished 4.2–4.4. Production overlay motion remains disabled; stage 5 is not implemented by this checkpoint.

## Verification

Environment: macOS arm64 host, Node v24.21.0, Bits UI 2.19.2, Playwright 1.63.0 Chromium. Browser evidence only, not native Windows/macOS, screen-reader, or activation-latency evidence.

- `npm run check`: 0 errors, 0 warnings.
- `npm run build`: pass, 745 modules; CSS 28.30 kB and JS 206.22 kB (63.88 kB gzip). Popover positioning/focus/presence code now participates in the production bundle; no production dependency added.
- `npm run test:browser`: 96/96 pass.
- `npm run test:browser -- tests/browser/ActionPanel.integration.spec.ts --repeat-each=2`: 70/70 pass.
- `npx @sveltejs/mcp svelte-autofixer <file> --svelte-version 5`: App, ActionPanel, ProtocolView, LauncherFooter, and ActionScope analyzed; zero issues. Raw output: `svelte-analysis.txt`. Existing scroll/focus/IPC effects are intentional side effects; DOM refs remain bindable. No suggested orchestration rewrite applied.
- `git diff --check`: pass. No staged files.
- Production bundle search for `dangoFixture`, `fixture-only`, `Clipboard preview`, and `fixture/application`: no matches.
- Source inspection: no custom Arrow/Enter selection or confirmation loop in ActionPanel. Bits owns ranking-independent selection, scrolling, positioning, Escape dismissal, outside dismissal, and focus lifecycle.

## Coverage

`ActionPanel.spec.ts` proves the separate portaled Command scope without a search field. `ActionPanel.integration.spec.ts` has 35 production-integration checks:

- Root/list/form/detail origins; Ctrl+K and Cmd+K, first action, non-wrapping arrows, hover-independent selection, exactly-once invocation and parent selection preservation.
- Actual trigger pointer/keyboard activation; appropriate search/form/detail focus restoration; empty action lists and removal of actions while open.
- 24-action list scrolling to its final option; 720×400 and 480×300 bounds in both palettes.
- Both fast and held outside presses: parent selection, activation, and focus stay unchanged even if the resulting click arrives after panel removal.
- Immediate Escape/Enter followed by held repeats across all four origins, without parent invocation, cancellation, or dismissal; reopen starts on the first action.
- Closed-but-present content rejects fresh Escape/Enter and clicks; outgoing items disabled; focus already returned before presence finishes.
- Carried failures survive opening/Escape dismissal and clear on subsequent typing.
- Actual production portal inherits live typography, radius, width, surface, selection, hover, and text overrides. Independent role checks also run in `theme.spec.ts`; the earlier development token probe is no longer the sole action-panel evidence.

All integration tests request reduced motion. Two presence tests inject a test-only 180ms CSS exit and pause it through the Web Animations API, allowing deterministic assertions before explicitly finishing it. This tests the boundary without enabling production transitions or using sleeps as evidence. Production portal reports zero running animations.

## Adapter findings

The initial outside-click test failed before the fix. Changing `data-pointer` on Popover.Content recycled the installed primitive's ref/listeners during pointer interaction and cancelled its deferred outside callback. Keeping that marker on the inner Command fixes dismissal. A narrow capture adapter consumes the outside press's mousedown/click without blocking pointerdown or implementing dismissal; Bits' delayed hook alone cannot stop an already-dispatched parent click. Temporary dependency instrumentation was restored; no node_modules/package change retained by this continuation.

Escape must reach Bits' document listener, which prevents the original event and gives its hook a clone. Launcher window handlers honor `defaultPrevented`. Closed presence and held closing-key repeats are isolated by a capture guard, not a second navigation implementation. See design.md, “Installed primitive integration (stage 4)”.

## Visual evidence

- `dark-720-actions.png`, `light-720-actions.png`
- `dark-480-actions.png`, `light-480-actions.png`

Screenshots inspected: final selected action visible, portal bounded, footer Actions trigger reachable, parent row remains selected, no corner clipping or horizontal overflow. Existing stage-2 palette values retained. Contrast refinement is not certified here; stage 5 owns it.

## Remaining boundaries

Stage 5 must add actual entrance/exit motion, busy feedback, reduced-motion/hidden-surface handling, contrast refinement, and repeat these lifecycle checks against the real transitions. Busy-surface override integration remains open. Native Windows/macOS visual, accessibility, scaling, and release activation/performance checks remain unchecked. Browser success does not certify NVDA or VoiceOver selected-option announcements.
